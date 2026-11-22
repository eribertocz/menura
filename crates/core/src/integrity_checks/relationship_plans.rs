//! The logical plans of the `key_uniqueness` and `orphan_keys` checks of a relationship
//! (CHK-6.1, CHK-6.2, CHK-4.2).

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_model::expression::{
    AggregateCall, BinaryOperator, Expression, IntegerLiteral, Literal,
};
use menura_model::internal_model::{Dataset, InternalModel, NormalizedName, Relationship};
use menura_representations::logical_plan::{
    AggregateComponent, JoinCondition, JoinKind, NamedExpression, Operation, OutputSchema,
    PlanNode, SchemaColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;

use super::key_columns::{KeyColumn, STAGE, key_columns};
use super::plan_nodes::{all_not_null, filter, is_null, project, reference, scan};
use super::result_names::added_column_name;
use crate::compile::InternalError;

/// The name CHK-6.1 gives the number of rows of a group.
const ROW_COUNT: &str = "row_count";

/// The plan of `key_uniqueness` (CHK-6.1): the groups of the `to_columns` values of `to` whose
/// values are all non-NULL and that have more than one row. Its columns are the `to_columns`
/// fields in pair order, then `row_count`, a 64-bit signed integer.
pub(super) fn key_uniqueness_plan(
    relationship: &Relationship,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, InternalError> {
    let origin = Origin::Model(relationship.location.clone());
    let to = dataset(model, &relationship.to)?;
    let keys = key_columns(to, &relationship.to_columns, catalog)?;
    let complete = filter(
        scan(&to.name, &keys, &origin),
        all_not_null(&keys)?,
        &origin,
    );
    let count = synthesized_count_column(&keys);
    let grouped = group_count(complete, &count, &origin)?;
    let repeated = filter(grouped, more_than_one(&count)?, &origin);
    let field_columns: Vec<String> = keys.iter().map(|key| key.result.clone()).collect();
    let mut columns: Vec<(ColumnName, String)> = keys
        .iter()
        .map(|key| (key.scan.clone(), key.result.clone()))
        .collect();
    columns.push((count, added_column_name(ROW_COUNT, &field_columns)));
    project(repeated, &columns, &origin)
}

/// The plan of `orphan_keys` (CHK-6.2): the rows of `from` whose `from_columns` values are all
/// non-NULL and that no row of `to` matches pair by pair, as a left join whose right side is
/// missing. Its columns are the `from_columns` fields in pair order.
pub(super) fn orphan_keys_plan(
    relationship: &Relationship,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, InternalError> {
    let origin = Origin::Model(relationship.location.clone());
    let from = dataset(model, &relationship.from)?;
    let to = dataset(model, &relationship.to)?;
    let from_keys = key_columns(from, &relationship.from_columns, catalog)?;
    let to_keys = key_columns(to, &relationship.to_columns, catalog)?;
    let complete = filter(
        scan(&from.name, &from_keys, &origin),
        all_not_null(&from_keys)?,
        &origin,
    );
    let prefix = ColumnPrefix::new(&format!("{}.", relationship.name.as_str()));
    let joined = left_join(
        complete,
        &to.name,
        &to_keys,
        &from_keys,
        relationship,
        &prefix,
    );
    let first_right = to_keys
        .first()
        .map(|key| prefix.column(key.scan.as_str()))
        .ok_or_else(|| InternalError::new(STAGE, "a relationship has no column pair"))?;
    let orphans = filter(joined, is_null(&first_right, false)?, &origin);
    let columns: Vec<(ColumnName, String)> = from_keys
        .iter()
        .map(|key| (key.scan.clone(), key.result.clone()))
        .collect();
    project(orphans, &columns, &origin)
}

/// The left `Join` of `left` with the `Scan` of `to` along `relationship`, every column of the
/// right side under `prefix` and nullable (E6-4.12, PLN-1).
fn left_join(
    left: PlanNode,
    to: &NormalizedName,
    to_keys: &[KeyColumn],
    from_keys: &[KeyColumn],
    relationship: &Relationship,
    prefix: &ColumnPrefix,
) -> PlanNode {
    let origin = Origin::Model(relationship.location.clone());
    let right = scan(to, to_keys, &origin);
    let mut schema: Vec<SchemaColumn> = left.schema().columns().to_vec();
    let right_schema: Vec<SchemaColumn> = right
        .schema()
        .columns()
        .iter()
        .map(|column| SchemaColumn {
            name: prefix.column(column.name.as_str()),
            logical_type: column.logical_type.clone(),
            nullable: true,
        })
        .collect();
    schema.extend(right_schema.iter().cloned());
    let right = PlanNode::new(
        right.operation().clone(),
        OutputSchema::new(right_schema),
        origin.clone(),
    );
    let on = from_keys
        .iter()
        .zip(to_keys)
        .map(|(from_key, to_key)| JoinCondition {
            left: from_key.scan.clone(),
            right: prefix.column(to_key.scan.as_str()),
        })
        .collect();
    PlanNode::new(
        Operation::Join {
            left: Box::new(left),
            right: Box::new(right),
            relationship: relationship.name.clone(),
            kind: JoinKind::Left,
            on,
            right_prefix: prefix.clone(),
        },
        OutputSchema::new(schema),
        origin,
    )
}

/// The `Aggregate` of `input` grouped by every column of `input`, with `COUNT(*)` as `count`.
fn group_count(
    input: PlanNode,
    count: &ColumnName,
    origin: &Origin,
) -> Result<PlanNode, InternalError> {
    let keys = input
        .schema()
        .columns()
        .iter()
        .map(|column| {
            Ok(NamedExpression {
                name: column.name.clone(),
                expression: reference(&column.name)?,
                derived: None,
            })
        })
        .collect::<Result<Vec<_>, InternalError>>()?;
    let mut schema: Vec<SchemaColumn> = input.schema().columns().to_vec();
    schema.push(SchemaColumn {
        name: count.clone(),
        logical_type: LogicalType::Integer {
            bits: IntegerBits::SixtyFour,
            signed: true,
        },
        nullable: false,
    });
    Ok(PlanNode::new(
        Operation::Aggregate {
            input: Box::new(input),
            keys,
            components: vec![AggregateComponent {
                name: count.clone(),
                call: AggregateCall::CountRows,
                filter: None,
                recomposition: None,
            }],
        },
        OutputSchema::new(schema),
        origin.clone(),
    ))
}

/// `count > 1`.
fn more_than_one(count: &ColumnName) -> Result<Expression, InternalError> {
    let one = IntegerLiteral::new("1")
        .ok_or_else(|| InternalError::new(STAGE, "the literal 1 is not an integer"))?;
    Ok(Expression::Binary {
        operator: BinaryOperator::Greater,
        left: Box::new(reference(count)?),
        right: Box::new(Expression::Literal(Literal::Integer(one))),
    })
}

/// The synthesized column of the row count: `menura.row_count`, followed by as many `.` as
/// needed so that it names no key column (E6-4.12).
fn synthesized_count_column(keys: &[KeyColumn]) -> ColumnName {
    let mut name = String::from("menura.row_count");
    while keys.iter().any(|key| key.scan.as_str() == name) {
        name.push('.');
    }
    ColumnName::new(&name)
}

fn dataset<'model>(
    model: &'model InternalModel,
    name: &NormalizedName,
) -> Result<&'model Dataset, InternalError> {
    model.dataset(name).ok_or_else(|| {
        InternalError::new(
            STAGE,
            format!("a relationship names the unknown dataset {}", name.as_str()),
        )
    })
}
