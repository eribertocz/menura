//! The plan of the `summary` check (CHK-6.6): the base aggregate, recomputed exactly as the
//! build query computes it, full outer joined with the summary rows in the same range on every
//! dimension, keeping the rows where a side is missing or a measure differs.

use menura_model::expression::{BinaryOperator, CaseBranch, Expression, Literal, ScalarFunction};
use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{
    JoinCondition, NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;

use super::plan_nodes::{filter, is_null, reference};
use super::result_names::added_column_name;
use super::summary_base::{SummaryField, SummaryParts, base_aggregate, defect, field_column};
use super::typed_columns::typed_column;
use crate::compile::InternalError;

/// The plan of the `summary` check of `parts` (CHK-6.6). Its columns are the dimensions in
/// field order, then `base.<measure>` and `summary.<measure>` for each measure in field order,
/// then `violation`, each added name made unique as CHK-2.6 states.
pub(super) fn summary_check_plan(parts: &SummaryParts<'_>) -> Result<PlanNode, InternalError> {
    let origin = parts.origin();
    let fields = parts.fields()?;
    let base = base_aggregate(parts)?;
    let prefix = summary_prefix(base.schema());
    let joined = full_join(
        base,
        stored_rows(parts, &fields, &origin)?,
        &fields,
        &prefix,
    );
    let compared = compared(joined, parts, &fields, &prefix)?;
    let violation = compared
        .schema()
        .columns()
        .last()
        .map(|column| column.name.clone())
        .ok_or_else(|| defect(parts.summary, "has a check without columns"))?;
    Ok(filter(compared, is_null(&violation, true)?, &origin))
}

/// The summary rows whose coverage day is in the bound period range (CHK-7.4), with every
/// summary field.
fn stored_rows(
    parts: &SummaryParts<'_>,
    fields: &[SummaryField<'_>],
    origin: &Origin,
) -> Result<PlanNode, InternalError> {
    let schema = fields
        .iter()
        .map(|field| {
            let declared = parts
                .dataset
                .field(field.name())
                .ok_or_else(|| defect(parts.summary, "has a field its dataset does not declare"))?;
            field_column(parts.summary, declared)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let scan = PlanNode::new(
        Operation::Scan {
            dataset: parts.dataset.name.clone(),
            row_identity: None,
        },
        OutputSchema::new(schema),
        origin.clone(),
    );
    let schema = scan.schema().clone();
    Ok(PlanNode::new(
        Operation::PeriodRange {
            input: Box::new(scan),
            coverage: ColumnName::new(parts.summary.coverage_field.as_str()),
        },
        schema,
        origin.clone(),
    ))
}

/// The `FullJoin` of the base aggregate `base` and the summary rows `stored` on every
/// dimension, the columns of `stored` under `prefix`; every column is nullable.
fn full_join(
    base: PlanNode,
    stored: PlanNode,
    fields: &[SummaryField<'_>],
    prefix: &ColumnPrefix,
) -> PlanNode {
    let origin = base.origin().clone();
    let nullable = |column: &SchemaColumn, name: ColumnName| SchemaColumn {
        name,
        logical_type: column.logical_type.clone(),
        nullable: true,
    };
    let mut schema: Vec<SchemaColumn> = base
        .schema()
        .columns()
        .iter()
        .map(|column| nullable(column, column.name.clone()))
        .collect();
    schema.extend(
        stored
            .schema()
            .columns()
            .iter()
            .map(|column| nullable(column, prefix.column(column.name.as_str()))),
    );
    let on = fields
        .iter()
        .filter(|field| matches!(field, SummaryField::Dimension(_)))
        .map(|field| JoinCondition {
            left: ColumnName::new(field.name().as_str()),
            right: prefix.column(field.name().as_str()),
        })
        .collect();
    PlanNode::new(
        Operation::FullJoin {
            left: Box::new(base),
            right: Box::new(stored),
            on,
            right_prefix: prefix.clone(),
        },
        OutputSchema::new(schema),
        origin,
    )
}

/// The `Project` of the compared columns of `joined` (CHK-6.6).
fn compared(
    joined: PlanNode,
    parts: &SummaryParts<'_>,
    fields: &[SummaryField<'_>],
    prefix: &ColumnPrefix,
) -> Result<PlanNode, InternalError> {
    let base = |name: &NormalizedName| ColumnName::new(name.as_str());
    let stored = |name: &NormalizedName| prefix.column(name.as_str());
    let dimension_names: Vec<String> = fields
        .iter()
        .filter(|field| matches!(field, SummaryField::Dimension(_)))
        .map(|field| field.name().as_str().to_owned())
        .collect();
    let mut columns: Vec<(String, Expression)> = Vec::new();
    for field in fields {
        if let SummaryField::Dimension(dimension) = field {
            let either = Expression::Function {
                function: ScalarFunction::Coalesce,
                arguments: vec![
                    reference(&base(&dimension.field))?,
                    reference(&stored(&dimension.field))?,
                ],
            };
            columns.push((dimension.field.as_str().to_owned(), either));
        }
    }
    let mut differences = Vec::new();
    for field in fields {
        if let SummaryField::Measure(measure) = field {
            let name = measure.field.as_str();
            let added = |side: &str| added_column_name(&format!("{side}.{name}"), &dimension_names);
            columns.push((added("base"), reference(&base(&measure.field))?));
            columns.push((added("summary"), reference(&stored(&measure.field))?));
            differences.push(Expression::IsDistinctFrom {
                left: Box::new(reference(&base(&measure.field))?),
                right: Box::new(reference(&stored(&measure.field))?),
                negated: false,
            });
        }
    }
    let coverage = &parts.summary.coverage_field;
    let violation = violation(&base(coverage), &stored(coverage), differences)?;
    columns.push((added_column_name("violation", &dimension_names), violation));
    project_expressions(joined, &columns)
}

/// `CASE WHEN stored IS NULL THEN 'missing_in_summary' WHEN base IS NULL THEN
/// 'missing_in_base' WHEN d1 OR d2 … THEN 'measure_differs' END`, where `base` and `stored`
/// are the coverage dimension of each side, never NULL on a side that exists (CHK-7.4).
fn violation(
    base: &ColumnName,
    stored: &ColumnName,
    differences: Vec<Expression>,
) -> Result<Expression, InternalError> {
    let text = |value: &str| Expression::Literal(Literal::Text(value.to_owned()));
    let mut differences = differences.into_iter();
    let first = differences
        .next()
        .ok_or_else(|| InternalError::new(super::key_columns::STAGE, "a summary has no measure"))?;
    let any_difference = differences.fold(first, |any, next| Expression::Binary {
        operator: BinaryOperator::Or,
        left: Box::new(any),
        right: Box::new(next),
    });
    Ok(Expression::Case {
        operand: None,
        branches: vec![
            CaseBranch {
                condition: is_null(stored, false)?,
                result: text("missing_in_summary"),
            },
            CaseBranch {
                condition: is_null(base, false)?,
                result: text("missing_in_base"),
            },
            CaseBranch {
                condition: any_difference,
                result: text("measure_differs"),
            },
        ],
        otherwise: None,
    })
}

/// The `Project` of `input` that computes each of `columns` (output name, expression), each
/// typed by the typer of E0.f.
fn project_expressions(
    input: PlanNode,
    columns: &[(String, Expression)],
) -> Result<PlanNode, InternalError> {
    let origin = input.origin().clone();
    let mut expressions = Vec::new();
    let mut schema = Vec::new();
    for (name, expression) in columns {
        let column = ColumnName::new(name);
        schema.push(typed_column(&column, expression, input.schema())?);
        expressions.push(NamedExpression {
            name: column,
            expression: expression.clone(),
            derived: None,
        });
    }
    Ok(PlanNode::new(
        Operation::Project {
            input: Box::new(input),
            expressions,
        },
        OutputSchema::new(schema),
        origin,
    ))
}

/// The shortest `menura.summary.` followed by one or more `.` that no column of `base` starts
/// with: the prefix of the summary columns in the `FullJoin` (E6-4.12).
fn summary_prefix(base: &OutputSchema) -> ColumnPrefix {
    let mut prefix = String::from("menura.summary.");
    while base
        .columns()
        .iter()
        .any(|column| column.name.as_str().starts_with(&prefix))
    {
        prefix.push('.');
    }
    ColumnPrefix::new(&prefix)
}
