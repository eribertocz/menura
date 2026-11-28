//! The step of a `Join` node (E4-3.2, E6-4.12, E6-4.13): the columns of both inputs, the right
//! one matched on the key pairs.

use menura_representations::logical_plan::{
    JoinCondition, JoinKind, OutputSchema, PlanNode, SchemaColumn, TemporalCoverage,
};
use menura_representations::names::{ColumnName, ColumnPrefix};

use crate::body::SelectBody;
use crate::coverage::{CoverageSide, coverage_conditions};
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::join_keys::{KeySide, key_equality};
use crate::steps::{StepWriter, error};

/// The inputs of a `Join` node (E4-3.2).
#[derive(Debug, Clone, Copy)]
pub(crate) struct JoinInputs<'input> {
    /// The left input: the home `Scan` or the previous `Join`.
    pub(crate) left: &'input PlanNode,
    /// The right input: the `Scan` of the join instance.
    pub(crate) right: &'input PlanNode,
    /// `left` or `inner`.
    pub(crate) kind: JoinKind,
    /// The key pairs, in the order written.
    pub(crate) on: &'input [JoinCondition],
    /// Along a temporal relationship, the coverage a version must also meet (VAL-2.8).
    pub(crate) temporal: Option<&'input TemporalCoverage>,
    /// The prefix of every column of `right`.
    pub(crate) right_prefix: &'input ColumnPrefix,
}

impl<'input> StepWriter<'input> {
    /// The steps of `left`, then of `right` (E6-4.12), then the step that selects every column
    /// of `node` from them, joined on the key pairs and, along a temporal relationship, on the
    /// coverage of the row's `from_time` (VAL-2.8, VAL-2.22): a row with k covering versions
    /// gives k rows, and under `left` a row with none gives one row whose right columns are
    /// NULL. A column of the right side of a left join is nullable (PLN-1).
    pub(crate) fn join(
        &mut self,
        node: &'input PlanNode,
        join: &JoinInputs<'input>,
    ) -> Result<SelectBody, GenerateError> {
        let JoinInputs {
            left,
            right,
            kind,
            on: keys,
            temporal,
            right_prefix,
        } = *join;
        if keys.is_empty() {
            return Err(error(InvariantViolation::JoinWithoutKeys, node));
        }
        let left_step = self.write(left)?;
        self.column_prefix = Some(right_prefix);
        let right_step = self.write(right);
        self.column_prefix = None;
        let right_step = right_step?;
        self.parameters.start_node(node.origin());
        let items = node
            .schema()
            .columns()
            .iter()
            .map(|column| joined_item(node, left.schema(), right.schema(), kind, column))
            .collect::<Result<Vec<_>, _>>()?;
        let conditions = keys
            .iter()
            .map(|key| {
                let left_column = key_column(node, left.schema(), &left_step.name, &key.left)?;
                let right_column = key_column(node, right.schema(), &right_step.name, &key.right)?;
                key_equality(left_column.side(), right_column.side())
                    .ok_or_else(|| error(InvariantViolation::IncompatibleJoinKeys, node))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut conditions = conditions;
        if let Some(coverage) = temporal {
            conditions.extend(coverage_conditions(
                node,
                coverage,
                CoverageSide {
                    schema: left.schema(),
                    qualifier: &left_step.name,
                },
                CoverageSide {
                    schema: right.schema(),
                    qualifier: &right_step.name,
                },
            )?);
        }
        let keyword = match kind {
            JoinKind::Left => "LEFT JOIN",
            JoinKind::Inner => "INNER JOIN",
        };
        Ok(SelectBody {
            items,
            from: left_step.name,
            clauses: vec![format!(
                "{keyword} {} ON {}",
                right_step.name,
                conjunction(&conditions)
            )],
        })
    }
}

/// A key column, qualified by the name of its step, with its type.
struct KeyColumn<'input> {
    sql: String,
    column: &'input SchemaColumn,
}

impl KeyColumn<'_> {
    fn side(&self) -> KeySide<'_> {
        KeySide {
            sql: &self.sql,
            logical_type: &self.column.logical_type,
        }
    }
}

/// The key column `name` of `schema`, the schema of the step `step`.
fn key_column<'input>(
    node: &PlanNode,
    schema: &'input OutputSchema,
    step: &str,
    name: &ColumnName,
) -> Result<KeyColumn<'input>, GenerateError> {
    let column = schema
        .columns()
        .iter()
        .find(|column| &column.name == name)
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
    Ok(KeyColumn {
        sql: format!("{step}.{}", quote(name.as_str())),
        column,
    })
}

/// The select item of the `Join` column `column`: the column of the same name of exactly one
/// input. A column of the right side of a left join must be nullable (PLN-1).
fn joined_item(
    node: &PlanNode,
    left: &OutputSchema,
    right: &OutputSchema,
    kind: JoinKind,
    column: &SchemaColumn,
) -> Result<String, GenerateError> {
    let has = |schema: &OutputSchema| {
        schema
            .columns()
            .iter()
            .any(|candidate| candidate.name == column.name)
    };
    match (has(left), has(right)) {
        (false, true) if kind == JoinKind::Left && !column.nullable => {
            Err(error(InvariantViolation::NonNullableJoinedColumn, node))
        }
        (true, false) | (false, true) => Ok(quote(column.name.as_str())),
        (true, true) => Err(error(InvariantViolation::AmbiguousColumn, node)),
        (false, false) => Err(error(InvariantViolation::UnknownColumn, node)),
    }
}

/// `condition`, or `(a) AND (b)` for several.
fn conjunction(conditions: &[String]) -> String {
    match conditions {
        [single] => single.clone(),
        _ => conditions
            .iter()
            .map(|condition| format!("({condition})"))
            .collect::<Vec<_>>()
            .join(" AND "),
    }
}
