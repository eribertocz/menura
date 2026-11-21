//! The steps of a `SemiJoin` node (E4-3.4, E4-6.5a, E6-4.14): keep a row of the input only if
//! it has at least one related row and every related row is visible.
//!
//! The steps of the input come first, then those of `related_all`, then those of
//! `related_visible` (E6-4.12). Two grouped steps follow: per related key, the number of rows
//! of `related_all` and the number of rows of `related_visible`. A row of the input is kept
//! when its key finds a group of `related_all` and the same number of visible rows. Keys are
//! compared with `=`, so a NULL key never matches.

use menura_contracts::types::common::LogicalType;
use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{JoinCondition, Operation, OutputSchema, PlanNode};
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::join_keys::{KeySide, key_equality};
use crate::steps::{StepWriter, error};

/// The alias of the input row.
const ROW: &str = "\"menura_row\"";
/// The alias of the grouped step of `related_all`.
const RELATED: &str = "\"menura_related\"";
/// The alias of the grouped step of `related_visible`.
const VISIBLE: &str = "\"menura_visible\"";
/// The number of related rows of one key.
const ROWS: &str = "\"menura.rows\"";

/// The operands of one `SemiJoin` node.
struct SemiJoinInputs<'input> {
    /// The rows of the dataset whose rows are kept or removed.
    input: &'input PlanNode,
    /// The relationship of the step.
    relationship: &'input NormalizedName,
    /// The key equalities: an `input` column equals a related column.
    on: &'input [JoinCondition],
    /// The unfiltered `Scan` of the related dataset.
    related_all: &'input PlanNode,
    /// The plan of the visible rows of the related dataset.
    related_visible: &'input PlanNode,
}

/// The operands of `node`, when it is a `SemiJoin`.
fn operands(node: &PlanNode) -> Option<SemiJoinInputs<'_>> {
    match node.operation() {
        Operation::SemiJoin {
            input,
            relationship,
            on,
            related_all,
            related_visible,
        } => Some(SemiJoinInputs {
            input,
            relationship,
            on,
            related_all,
            related_visible,
        }),
        _ => None,
    }
}

impl<'input> StepWriter<'input> {
    /// The steps of the operands of the `SemiJoin` `node`, in operand order, then its body.
    pub(crate) fn semi_join(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<SelectBody, GenerateError> {
        let inputs =
            &operands(node).ok_or_else(|| error(InvariantViolation::NodeOutsideSlice, node))?;
        self.check(node, inputs)?;
        let input = self.write(inputs.input)?;
        let all = self.write(inputs.related_all)?;
        let visible = self.write(inputs.related_visible)?;
        let all_counts = self.push_step("related", key_counts(inputs.on, &all.name));
        let visible_counts = self.push_step("visible", key_counts(inputs.on, &visible.name));
        let input_schema = inputs.input.schema();
        let all_condition = key_condition(
            node,
            inputs.on,
            input_schema,
            inputs.related_all.schema(),
            RELATED,
        )?;
        let visible_condition = key_condition(
            node,
            inputs.on,
            input_schema,
            inputs.related_visible.schema(),
            VISIBLE,
        )?;
        Ok(SelectBody {
            items: kept_columns(node, input_schema)?,
            from: format!("{} AS {ROW}", input.name),
            clauses: vec![
                format!("JOIN {all_counts} AS {RELATED} ON {all_condition}"),
                format!("LEFT JOIN {visible_counts} AS {VISIBLE} ON {visible_condition}"),
                format!("WHERE {RELATED}.{ROWS} = {VISIBLE}.{ROWS}"),
            ],
        })
    }

    /// Checks that the relationship exists and that every key column is a column of its side.
    fn check(&self, node: &PlanNode, inputs: &SemiJoinInputs<'input>) -> Result<(), GenerateError> {
        let known = self
            .environment
            .model
            .parts()
            .relationships
            .iter()
            .any(|relationship| &relationship.name == inputs.relationship);
        if !known {
            return Err(error(InvariantViolation::UnknownRelationship, node));
        }
        if inputs.on.is_empty() {
            return Err(error(InvariantViolation::EmptyJoinCondition, node));
        }
        for condition in inputs.on {
            let present = has_column(inputs.input.schema(), &condition.left)
                && has_column(inputs.related_all.schema(), &condition.right)
                && has_column(inputs.related_visible.schema(), &condition.right);
            if !present {
                return Err(error(InvariantViolation::UnknownColumn, node));
            }
        }
        Ok(())
    }

    /// Adds the step `body` of `kind` and returns its quoted name.
    fn push_step(&mut self, kind: &str, body: SelectBody) -> String {
        let name = quote(&format!("menura_{}_{kind}", self.steps.len() + 1));
        self.steps.push((name.clone(), body));
        name
    }
}

/// Whether `schema` has the column `name`.
fn has_column(schema: &OutputSchema, name: &ColumnName) -> bool {
    schema.columns().iter().any(|column| &column.name == name)
}

/// The columns of `node`, each read from the input row.
fn kept_columns(node: &PlanNode, input: &OutputSchema) -> Result<Vec<String>, GenerateError> {
    node.schema()
        .columns()
        .iter()
        .map(|column| {
            if has_column(input, &column.name) {
                Ok(format!("{ROW}.{}", quote(column.name.as_str())))
            } else {
                Err(error(InvariantViolation::UnknownColumn, node))
            }
        })
        .collect()
}

/// The grouped step of `related`: its key columns, in the order of `on`, and the number of
/// its rows per key.
fn key_counts(on: &[JoinCondition], related: &str) -> SelectBody {
    let mut items: Vec<String> = on
        .iter()
        .enumerate()
        .map(|(index, condition)| {
            format!("{} AS {}", quote(condition.right.as_str()), key_name(index))
        })
        .collect();
    let groups: Vec<String> = (1..=items.len())
        .map(|ordinal| ordinal.to_string())
        .collect();
    items.push(format!("COUNT(*) AS {ROWS}"));
    SelectBody {
        items,
        from: related.to_owned(),
        clauses: vec![format!("GROUP BY {}", groups.join(", "))],
    }
}

/// `row.left_1 = alias.key_1 AND …`, one equality per condition, in the order written; a
/// pair of two types is compared in their common type (E6-4.13). The key columns of `alias`
/// have the types of their columns in `related`.
fn key_condition(
    node: &PlanNode,
    on: &[JoinCondition],
    input: &OutputSchema,
    related: &OutputSchema,
    alias: &str,
) -> Result<String, GenerateError> {
    let equalities = on
        .iter()
        .enumerate()
        .map(|(index, condition)| {
            let left_type = column_type(node, input, &condition.left)?;
            let right_type = column_type(node, related, &condition.right)?;
            let left_sql = format!("{ROW}.{}", quote(condition.left.as_str()));
            let right_sql = format!("{alias}.{}", key_name(index));
            key_equality(
                KeySide {
                    sql: &left_sql,
                    logical_type: left_type,
                },
                KeySide {
                    sql: &right_sql,
                    logical_type: right_type,
                },
            )
            .ok_or_else(|| error(InvariantViolation::IncompatibleJoinKeys, node))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(equalities.join(" AND "))
}

/// The logical type of the column `name` of `schema`.
fn column_type<'schema>(
    node: &PlanNode,
    schema: &'schema OutputSchema,
    name: &ColumnName,
) -> Result<&'schema LogicalType, GenerateError> {
    schema
        .columns()
        .iter()
        .find(|column| &column.name == name)
        .map(|column| &column.logical_type)
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))
}

/// The name of the key column at `index`, from 0, of a grouped step.
fn key_name(index: usize) -> String {
    quote(&format!("menura.key_{}", index + 1))
}
