//! The steps of a `Join` along a temporal relationship in `ClickHouse` (VAL-2.22, VAL-2.23,
//! E6-4.14). A join condition other than equalities is not written in `ON`: the left rows are
//! numbered, each row is joined on its keys with every version of its entity, and the coverage
//! of its `from_time` is computed per joined row. A covered row keeps its version; a left row
//! that no version covers keeps one row whose right columns are NULL under `left`, and none
//! under `inner`. `ASOF JOIN` is never used, because it keeps one version where VAL-2.8 keeps
//! every covering one (VAL-2.23). Each step reads the one before it once.

use menura_representations::logical_plan::{JoinKind, OutputSchema, PlanNode, SchemaColumn};

use crate::body::SelectBody;
use crate::coverage::{CoverageSide, coverage_conditions};
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::join::{JoinInputs, joined_item};
use crate::join_keys::{KeySide, key_equality};
use crate::steps::{StepWriter, error};

/// The alias of a left row.
const ROW: &str = "\"menura_row\"";
/// The alias of a version.
const VERSION: &str = "\"menura_version\"";

/// The names of the columns the steps add, none of which is a column of either side.
struct Added {
    row: String,
    covers: String,
    covered: String,
    rank: String,
}

impl<'input> StepWriter<'input> {
    /// The steps of `join.left`, then of `join.right`, then those that number, join and rank
    /// the rows, and the body that keeps the covered rows and one NULL row per uncovered left
    /// row under `left`.
    pub(crate) fn temporal_join(
        &mut self,
        node: &'input PlanNode,
        join: &JoinInputs<'input>,
        left_step: &str,
        right_step: &str,
    ) -> Result<SelectBody, GenerateError> {
        let added = added_names([join.left.schema(), join.right.schema()]);
        let rows = self.numbered_rows(join, left_step, &added);
        let joined = self.joined_versions(node, join, &rows, right_step, &added)?;
        let ranked = self.ranked_versions(join, &joined, &added);
        let items = node
            .schema()
            .columns()
            .iter()
            .map(|column| temporal_item(node, join, column, &added))
            .collect::<Result<Vec<_>, _>>()?;
        let kept = match join.kind {
            JoinKind::Left => format!(
                "WHERE {covers} = 1 OR ({covered} = 0 AND {rank} = 1)",
                covers = added.covers,
                covered = added.covered,
                rank = added.rank
            ),
            JoinKind::Inner => format!("WHERE {} = 1", added.covers),
        };
        Ok(SelectBody {
            items,
            from: ranked,
            clauses: vec![kept],
        })
    }

    /// The step of the left rows, each with its row number.
    fn numbered_rows(&mut self, join: &JoinInputs<'_>, left_step: &str, added: &Added) -> String {
        let mut items = quoted(join.left.schema());
        items.push(format!("ROW_NUMBER() OVER () AS {}", added.row));
        self.push_step(
            "rows",
            SelectBody {
                items,
                from: left_step.to_owned(),
                clauses: Vec::new(),
            },
        )
    }

    /// The step of each numbered left row joined on its keys with every version of its
    /// entity, and whether the version covers its `from_time`; a row without any version keeps
    /// one row whose coverage is 0.
    fn joined_versions(
        &mut self,
        node: &PlanNode,
        join: &JoinInputs<'_>,
        rows: &str,
        right_step: &str,
        added: &Added,
    ) -> Result<String, GenerateError> {
        let coverage = join
            .temporal
            .ok_or_else(|| error(InvariantViolation::UnknownRelationship, node))?;
        let (left, right) = (join.left.schema(), join.right.schema());
        let keys = key_conditions(node, join, left, right)?;
        let covering = coverage_conditions(
            node,
            coverage,
            CoverageSide {
                schema: left,
                qualifier: ROW,
            },
            CoverageSide {
                schema: right,
                qualifier: VERSION,
            },
        )?;
        let mut items: Vec<String> = quoted(left)
            .iter()
            .chain([&added.row])
            .map(|column| format!("{ROW}.{column}"))
            .chain(
                quoted(right)
                    .iter()
                    .map(|column| format!("{VERSION}.{column}")),
            )
            .collect();
        items.push(format!(
            "CASE WHEN {} THEN 1 ELSE 0 END AS {}",
            covering.join(" AND "),
            added.covers
        ));
        Ok(self.push_step(
            "versions",
            SelectBody {
                items,
                from: format!("{rows} AS {ROW}"),
                clauses: vec![format!(
                    "LEFT JOIN {right_step} AS {VERSION} ON {}",
                    keys.join(" AND ")
                )],
            },
        ))
    }

    /// The step of the joined rows with, per left row, whether any version covers it and the
    /// rank of each of its rows.
    fn ranked_versions(&mut self, join: &JoinInputs<'_>, joined: &str, added: &Added) -> String {
        let mut items: Vec<String> = quoted(join.left.schema())
            .into_iter()
            .chain(quoted(join.right.schema()))
            .collect();
        items.push(added.covers.clone());
        items.push(format!(
            "MAX({}) OVER (PARTITION BY {}) AS {}",
            added.covers, added.row, added.covered
        ));
        items.push(format!(
            "ROW_NUMBER() OVER (PARTITION BY {}) AS {}",
            added.row, added.rank
        ));
        self.push_step(
            "covered",
            SelectBody {
                items,
                from: joined.to_owned(),
                clauses: Vec::new(),
            },
        )
    }
}

/// The select item of the `Join` column `column`: a left column as it is, a right column
/// NULL on a row whose version does not cover it.
fn temporal_item(
    node: &PlanNode,
    join: &JoinInputs<'_>,
    column: &SchemaColumn,
    added: &Added,
) -> Result<String, GenerateError> {
    let item = joined_item(
        node,
        join.left.schema(),
        join.right.schema(),
        join.kind,
        column,
    )?;
    let is_right = join
        .right
        .schema()
        .columns()
        .iter()
        .any(|candidate| candidate.name == column.name);
    if is_right {
        Ok(format!(
            "CASE WHEN {} = 1 THEN {item} ELSE NULL END AS {item}",
            added.covers
        ))
    } else {
        Ok(item)
    }
}

/// The quoted names of the columns of `schema`.
fn quoted(schema: &OutputSchema) -> Vec<String> {
    schema
        .columns()
        .iter()
        .map(|column| quote(column.name.as_str()))
        .collect()
}

/// The quoted names of the columns the steps add: each `menura.` name followed by as many `.`
/// as make it differ from every column of `schemas` (E6-4.12).
fn added_names(schemas: [&OutputSchema; 2]) -> Added {
    let unique = |base: &str| {
        let mut name = String::from(base);
        while schemas.iter().any(|schema| {
            schema
                .columns()
                .iter()
                .any(|column| column.name.as_str() == name)
        }) {
            name.push('.');
        }
        quote(&name)
    };
    Added {
        row: unique("menura.row"),
        covers: unique("menura.covers"),
        covered: unique("menura.covered"),
        rank: unique("menura.rank"),
    }
}

/// `row.left = version.right`, one equality per key pair of `join`, in the order written; a
/// pair of two types is compared in their common type (E6-4.13).
fn key_conditions(
    node: &PlanNode,
    join: &JoinInputs<'_>,
    left: &OutputSchema,
    right: &OutputSchema,
) -> Result<Vec<String>, GenerateError> {
    join.on
        .iter()
        .map(|condition| {
            let left_type = column_type(node, left, condition.left.as_str())?;
            let right_type = column_type(node, right, condition.right.as_str())?;
            let left_sql = format!("{ROW}.{}", quote(condition.left.as_str()));
            let right_sql = format!("{VERSION}.{}", quote(condition.right.as_str()));
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
        .collect()
}

/// The logical type of the column `name` of `schema`.
fn column_type<'schema>(
    node: &PlanNode,
    schema: &'schema OutputSchema,
    name: &str,
) -> Result<&'schema menura_contracts::types::common::LogicalType, GenerateError> {
    schema
        .columns()
        .iter()
        .find(|column| column.name.as_str() == name)
        .map(|column| &column.logical_type)
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))
}
