//! The steps of a `SemiJoin` along a temporal relationship (E4-3.4, VAL-2.17, E6-4.14): keep a
//! row of the input only if at least one version of the related dataset covers its `from_time`
//! and every covering version is visible.
//!
//! The input rows are numbered, and each step reads the one before it once. Each row is joined
//! with the versions of its key in `related_all`, and the covering ones are counted per row; one
//! row per number is kept with that count. The same is done with `related_visible`. A row is
//! kept when it has at least one covering version and as many covering versions are visible.
//! Keys are compared with `=`, so a NULL key never matches, and a NULL `from_time` is covered
//! by no version (VAL-2.7).

use menura_representations::logical_plan::{
    JoinCondition, OutputSchema, PlanNode, TemporalCoverage,
};

use crate::body::SelectBody;
use crate::coverage::{CoverageSide, coverage_conditions};
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::join_keys::{KeySide, key_equality};
use crate::steps::{StepWriter, error};

/// The alias of an input row.
const ROW: &str = "\"menura_row\"";
/// The alias of a version.
const VERSION: &str = "\"menura_version\"";

/// The operands of a temporal `SemiJoin`.
pub(crate) struct TemporalSemiJoin<'input> {
    /// The rows of the dataset whose rows are kept or removed.
    pub(crate) input: &'input PlanNode,
    /// The key equalities: an `input` column equals a related column.
    pub(crate) on: &'input [JoinCondition],
    /// The coverage condition.
    pub(crate) coverage: &'input TemporalCoverage,
    /// The unfiltered `Scan` of the related dataset.
    pub(crate) related_all: &'input PlanNode,
    /// The plan of the visible rows of the related dataset.
    pub(crate) related_visible: &'input PlanNode,
}

/// The names of the columns the steps add, none of which is a column of the input.
struct Added {
    row: String,
    covers: String,
    related: String,
    visible: String,
    rank: String,
}

impl<'input> StepWriter<'input> {
    /// The steps of the operands of `semi_join`, in operand order, then those that count the
    /// covering versions of each row, then the body that keeps the visible rows.
    pub(crate) fn temporal_semi_join(
        &mut self,
        node: &'input PlanNode,
        semi_join: &TemporalSemiJoin<'input>,
    ) -> Result<SelectBody, GenerateError> {
        let input = self.write(semi_join.input)?;
        let all = self.write(semi_join.related_all)?;
        let visible = self.write(semi_join.related_visible)?;
        let schema = semi_join.input.schema();
        let added = added_names(schema);
        let columns: Vec<String> = schema
            .columns()
            .iter()
            .map(|column| quote(column.name.as_str()))
            .collect();
        let rows = self.numbered_input(&columns, &input.name, &added);
        let mut carried = columns.clone();
        carried.push(added.row.clone());
        let counted_all = self.counted_step(
            node,
            semi_join,
            Counted {
                rows: &rows,
                carried: &carried,
                versions: &all.name,
                versions_schema: semi_join.related_all.schema(),
                count: &added.related,
            },
            &added,
        )?;
        carried.push(added.related.clone());
        let counted_visible = self.counted_step(
            node,
            semi_join,
            Counted {
                rows: &counted_all,
                carried: &carried,
                versions: &visible.name,
                versions_schema: semi_join.related_visible.schema(),
                count: &added.visible,
            },
            &added,
        )?;
        Ok(SelectBody {
            items: columns,
            from: counted_visible,
            clauses: vec![format!(
                "WHERE {related} >= 1 AND {related} = {visible}",
                related = added.related,
                visible = added.visible
            )],
        })
    }

    /// The step of every row of `input` with its row number.
    fn numbered_input(&mut self, columns: &[String], input: &str, added: &Added) -> String {
        let mut items = columns.to_vec();
        items.push(format!("ROW_NUMBER() OVER () AS {}", added.row));
        self.push_step(
            "rows",
            SelectBody {
                items,
                from: input.to_owned(),
                clauses: Vec::new(),
            },
        )
    }

    /// The step of one row per row number of `counted.rows`, with the columns of
    /// `counted.carried` and, as `counted.count`, the number of versions of
    /// `counted.versions` that cover it.
    fn counted_step(
        &mut self,
        node: &'input PlanNode,
        semi_join: &TemporalSemiJoin<'input>,
        counted: Counted<'_>,
        added: &Added,
    ) -> Result<String, GenerateError> {
        let joined = self.covering_step(node, semi_join, counted, added)?;
        let mut items = counted.carried.to_vec();
        items.push(format!(
            "SUM({}) OVER (PARTITION BY {}) AS {}",
            added.covers, added.row, counted.count
        ));
        items.push(format!(
            "ROW_NUMBER() OVER (PARTITION BY {}) AS {}",
            added.row, added.rank
        ));
        let ranked = self.push_step(
            "covered",
            SelectBody {
                items,
                from: joined,
                clauses: Vec::new(),
            },
        );
        let mut items = counted.carried.to_vec();
        items.push(counted.count.to_owned());
        Ok(self.push_step(
            "counted",
            SelectBody {
                items,
                from: ranked,
                clauses: vec![format!("WHERE {} = 1", added.rank)],
            },
        ))
    }

    /// The step of each row of `counted.rows` joined on its keys with every version of
    /// `counted.versions`, and whether the version covers its `from_time`: a row without any
    /// version keeps one row whose coverage is 0.
    fn covering_step(
        &mut self,
        node: &'input PlanNode,
        semi_join: &TemporalSemiJoin<'input>,
        counted: Counted<'_>,
        added: &Added,
    ) -> Result<String, GenerateError> {
        let schema = semi_join.input.schema();
        let conditions = coverage_conditions(
            node,
            semi_join.coverage,
            CoverageSide {
                schema,
                qualifier: ROW,
            },
            CoverageSide {
                schema: counted.versions_schema,
                qualifier: VERSION,
            },
        )?;
        let keys = key_conditions(node, semi_join.on, schema, counted.versions_schema)?;
        let mut items: Vec<String> = counted
            .carried
            .iter()
            .map(|column| format!("{ROW}.{column}"))
            .collect();
        items.push(format!(
            "CASE WHEN {} THEN 1 ELSE 0 END AS {}",
            conditions.join(" AND "),
            added.covers
        ));
        Ok(self.push_step(
            "versions",
            SelectBody {
                items,
                from: format!("{} AS {ROW}", counted.rows),
                clauses: vec![format!(
                    "LEFT JOIN {} AS {VERSION} ON {}",
                    counted.versions,
                    keys.join(" AND ")
                )],
            },
        ))
    }
}

/// One counting of covering versions.
#[derive(Clone, Copy)]
struct Counted<'step> {
    /// The step of the numbered rows.
    rows: &'step str,
    /// The quoted columns of `rows` that the counted step keeps.
    carried: &'step [String],
    /// The step of the versions.
    versions: &'step str,
    /// Its columns.
    versions_schema: &'step OutputSchema,
    /// The quoted name of the count.
    count: &'step str,
}

/// The quoted names of the columns the steps add: each `menura.` name followed by as many `.`
/// as make it differ from every column of `schema` (E6-4.12).
fn added_names(schema: &OutputSchema) -> Added {
    let unique = |base: &str| {
        let mut name = String::from(base);
        while schema
            .columns()
            .iter()
            .any(|column| column.name.as_str() == name)
        {
            name.push('.');
        }
        quote(&name)
    };
    Added {
        row: unique("menura.row"),
        covers: unique("menura.covers"),
        related: unique("menura.related"),
        visible: unique("menura.visible"),
        rank: unique("menura.rank"),
    }
}

/// `row.left = version.right`, one equality per key pair, in the order written; a pair of two
/// types is compared in their common type (E6-4.13).
fn key_conditions(
    node: &PlanNode,
    on: &[JoinCondition],
    rows: &OutputSchema,
    versions: &OutputSchema,
) -> Result<Vec<String>, GenerateError> {
    if on.is_empty() {
        return Err(error(InvariantViolation::EmptyJoinCondition, node));
    }
    on.iter()
        .map(|condition| {
            let left_type = column_type(node, rows, condition.left.as_str())?;
            let right_type = column_type(node, versions, condition.right.as_str())?;
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
