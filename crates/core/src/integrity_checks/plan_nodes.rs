//! The plan nodes and expressions the checks are built from (CHK-4.2). Every node of a check
//! has model origin, so its constants are written as literals (E6-4.11).

use menura_model::expression::{BinaryOperator, Expression, Identifier, Reference};
use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{
    NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::key_columns::{KeyColumn, STAGE};
use crate::compile::InternalError;

/// The `Scan` of `dataset` that reads `columns`, each once, in order.
pub(super) fn scan(dataset: &NormalizedName, columns: &[KeyColumn], origin: &Origin) -> PlanNode {
    let mut schema: Vec<SchemaColumn> = Vec::new();
    for column in columns {
        if schema.iter().all(|known| known.name != column.scan) {
            schema.push(SchemaColumn {
                name: column.scan.clone(),
                logical_type: column.logical_type.clone(),
                nullable: column.nullable,
            });
        }
    }
    PlanNode::new(
        Operation::Scan {
            dataset: dataset.clone(),
            row_identity: None,
        },
        OutputSchema::new(schema),
        origin.clone(),
    )
}

/// The `Filter` of `input` on `predicate`, with the columns of `input`.
pub(super) fn filter(input: PlanNode, predicate: Expression, origin: &Origin) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Filter {
            input: Box::new(input),
            predicate,
        },
        schema,
        origin.clone(),
    )
}

/// The `Project` of `input` that renames each column of `columns` (input column, output
/// name), in order.
pub(super) fn project(
    input: PlanNode,
    columns: &[(ColumnName, String)],
    origin: &Origin,
) -> Result<PlanNode, InternalError> {
    let mut expressions = Vec::new();
    let mut schema = Vec::new();
    for (column, output) in columns {
        let read = input
            .schema()
            .columns()
            .iter()
            .find(|candidate| &candidate.name == column)
            .ok_or_else(|| unknown_column(column))?;
        let name = ColumnName::new(output);
        expressions.push(NamedExpression {
            name: name.clone(),
            expression: reference(column)?,
            derived: None,
        });
        schema.push(SchemaColumn {
            name,
            logical_type: read.logical_type.clone(),
            nullable: read.nullable,
        });
    }
    Ok(PlanNode::new(
        Operation::Project {
            input: Box::new(input),
            expressions,
        },
        OutputSchema::new(schema),
        origin.clone(),
    ))
}

/// The unqualified reference to the input column `column`, as a delimited identifier so that
/// it names the column exactly.
pub(super) fn reference(column: &ColumnName) -> Result<Expression, InternalError> {
    let name = Identifier::delimited(column.as_str())
        .ok_or_else(|| InternalError::new(STAGE, "a plan column has an empty name"))?;
    Ok(Expression::Reference(Reference {
        qualifier: None,
        name,
    }))
}

/// `column IS NULL`, or `column IS NOT NULL` when `negated`.
pub(super) fn is_null(column: &ColumnName, negated: bool) -> Result<Expression, InternalError> {
    Ok(Expression::IsNull {
        operand: Box::new(reference(column)?),
        negated,
    })
}

/// `c1 IS NOT NULL AND c2 IS NOT NULL AND …` over the distinct `Scan` columns of `columns`.
pub(super) fn all_not_null(columns: &[KeyColumn]) -> Result<Expression, InternalError> {
    let mut tested: Vec<&ColumnName> = Vec::new();
    for column in columns {
        if !tested.contains(&&column.scan) {
            tested.push(&column.scan);
        }
    }
    let mut conditions = tested.into_iter().map(|column| is_null(column, true));
    let first = conditions
        .next()
        .ok_or_else(|| InternalError::new(STAGE, "a relationship has no column pair"))??;
    conditions.try_fold(first, |all, condition| {
        Ok(Expression::Binary {
            operator: BinaryOperator::And,
            left: Box::new(all),
            right: Box::new(condition?),
        })
    })
}

fn unknown_column(column: &ColumnName) -> InternalError {
    InternalError::new(
        STAGE,
        format!("a check reads the unknown column {}", column.as_str()),
    )
}
