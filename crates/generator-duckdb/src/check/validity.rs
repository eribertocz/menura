//! The `population` check with `mode: validity` (CHK-6.3, POP-5): the rows whose interval is
//! inverted, and each row whose interval overlaps another row of the same entity. An inclusive
//! `valid_to` compares days; an exclusive one compares values and computes no day (VAL-1.4).

use menura_representations::logical_plan::PlanNode;
use menura_representations::names::ColumnName;

use super::sql::{
    aliased, all_equal, all_not_null, column, column_names, expect_names, known_columns,
    synthesized, violation,
};
use crate::body::{SelectBody, union_all};
use crate::error::GenerateError;
use crate::population::membership::{column_type, open_ended};
use crate::steps::{StepWriter, pass_through};
use menura_model::internal_model::ValidToBound;
use menura_representations::check_shapes::ValidityCheck;

/// The alias of a row that overlaps another.
const ROW: &str = "\"menura_row\"";
/// The alias of the other row.
const OTHER: &str = "\"menura_other\"";

/// The synthesized columns of the step that bounds every row, and how they compare.
struct BoundColumns {
    from: String,
    to: String,
    row: String,
    range: Range,
}

/// How the bounds of a row compare (CHK-6.3, VAL-1.4).
#[derive(Debug, Clone, Copy)]
struct Range {
    /// The step's name: `days` for days, `bounds` for values.
    step: &'static str,
    /// The base names of the columns of the start and the end.
    names: (&'static str, &'static str),
    /// `from` is before `to`, so that the row covers something.
    starts_before: &'static str,
    /// `from` is not before `to`: the row is `invalid_interval`.
    inverted: &'static str,
}

impl Range {
    /// Inclusive days (CHK-6.3), or exclusive values (VAL-1.4).
    fn of(bound: ValidToBound) -> Self {
        match bound {
            ValidToBound::Inclusive => Self {
                step: "days",
                names: ("menura.from_day", "menura.to_day"),
                starts_before: "<=",
                inverted: ">",
            },
            ValidToBound::Exclusive => Self {
                step: "bounds",
                names: ("menura.from", "menura.to"),
                starts_before: "<",
                inverted: ">=",
            },
        }
    }
}

impl<'input> StepWriter<'input> {
    /// The union of the `invalid_interval` rows and the `overlap` rows of `check` (CHK-6.3).
    /// Days are those of POP-3.5; each row has a row number, so two rows with equal values
    /// are two rows.
    pub(super) fn validity_check(
        &mut self,
        check: &ValidityCheck<'input>,
    ) -> Result<SelectBody, GenerateError> {
        let rows = check.rows;
        expect_names(&check.result_columns, check.entity.len() + 3, rows)?;
        let written = self.write(rows)?;
        self.parameters.start_node(rows.origin());
        let taken = column_names(rows);
        let range = Range::of(check.valid_to_bound);
        let columns = BoundColumns {
            from: synthesized(range.names.0, &taken),
            to: synthesized(range.names.1, &taken),
            row: synthesized("menura.row", &taken),
            range,
        };
        let days = self.days_step(rows, check, &columns, &written.name)?;
        let overlaps = self.push_step("overlaps", overlaps_body(check, &columns, &days));
        let mut fields: Vec<&ColumnName> = check.entity.iter().collect();
        fields.extend([&check.valid_from, &check.valid_to]);
        let values = known_columns(rows, &fields)?;
        let items = |name: &str| -> Vec<String> {
            let (field_names, added) = check.result_columns.split_at(values.len());
            values
                .iter()
                .zip(field_names)
                .map(|(value, field)| aliased(value, field))
                .chain(added.iter().map(|added| violation(name, added)))
                .collect()
        };
        let from = column(None, &columns.from);
        let to = column(None, &columns.to);
        let invalid = SelectBody {
            items: items("invalid_interval"),
            from: days.clone(),
            clauses: vec![format!(
                "WHERE {from} IS NOT NULL AND {to} IS NOT NULL AND {from} {} {to}",
                range.inverted
            )],
        };
        let row = column(None, &columns.row);
        let overlap = SelectBody {
            items: items("overlap"),
            from: days,
            clauses: vec![format!("WHERE {row} IN (SELECT {row} FROM {overlaps})")],
        };
        Ok(union_all(invalid, &[overlap]))
    }

    /// The step of every row with its bounds and its row number: its `valid_from` and
    /// `valid_to` days for an inclusive `valid_to`, their values for an exclusive one, an
    /// `open_end` read as NULL (VAL-1.2). The step is read three times and may be computed
    /// each time, so the rows are numbered in the order of every scanned column: only equal
    /// rows, which are interchangeable, can swap numbers between two readings.
    fn days_step(
        &mut self,
        rows: &'input PlanNode,
        check: &ValidityCheck<'input>,
        columns: &BoundColumns,
        written: &str,
    ) -> Result<String, GenerateError> {
        let mut items = pass_through(rows, rows.schema())?;
        for (field, name, open_end) in [
            (&check.valid_from, &columns.from, None),
            (&check.valid_to, &columns.to, check.open_end.as_ref()),
        ] {
            let logical_type = column_type(rows, rows, field)?;
            let value = open_ended(rows, &column(None, field.as_str()), open_end, logical_type)?;
            let bound = match check.valid_to_bound {
                ValidToBound::Inclusive => self.day_sql(rows, rows, &value, logical_type)?,
                ValidToBound::Exclusive => value,
            };
            items.push(aliased(&bound, name));
        }
        let order = column_names(rows)
            .iter()
            .map(|name| column(None, name.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        items.push(aliased(
            &format!("row_number() OVER (ORDER BY {order})"),
            &columns.row,
        ));
        let body = SelectBody {
            items,
            from: written.to_owned(),
            clauses: Vec::new(),
        };
        Ok(self.push_step(columns.range.step, body))
    }
}

/// The row numbers of the rows that overlap another row of the same entity: both have a
/// whole entity and a `valid_from`, neither is inverted, and each starts before the other
/// ends, on or before its last day for an inclusive `valid_to` (a NULL `valid_to` is
/// open-ended).
fn overlaps_body(check: &ValidityCheck<'_>, columns: &BoundColumns, days: &str) -> SelectBody {
    let entity = |alias: &str| -> Vec<String> {
        check
            .entity
            .iter()
            .map(|name| column(Some(alias), name.as_str()))
            .collect()
    };
    let mut conditions = vec![
        format!(
            "{} <> {}",
            column(Some(ROW), &columns.row),
            column(Some(OTHER), &columns.row)
        ),
        all_not_null(&entity(ROW)),
        all_equal(&entity(ROW), &entity(OTHER)),
    ];
    conditions.extend(range_conditions(columns));
    SelectBody {
        items: vec![aliased(&column(Some(ROW), &columns.row), &columns.row)],
        from: format!("{days} AS {ROW}"),
        clauses: vec![
            format!("JOIN {days} AS {OTHER} ON {}", conditions.join(" AND ")),
            "GROUP BY 1".to_owned(),
        ],
    }
}

/// The conditions on the bounds of two rows that overlap: each has a `valid_from` and is not
/// inverted, and each starts before the other ends (a NULL `valid_to` is open-ended).
fn range_conditions(columns: &BoundColumns) -> [String; 4] {
    let from = |alias: &str| column(Some(alias), &columns.from);
    let to = |alias: &str| column(Some(alias), &columns.to);
    let before = columns.range.starts_before;
    let starts_before_end = |start: &str, end: &str| {
        format!(
            "({} IS NULL OR {} {before} {})",
            to(end),
            from(start),
            to(end)
        )
    };
    let valid = |alias: &str| {
        format!(
            "{} IS NOT NULL AND {}",
            from(alias),
            starts_before_end(alias, alias)
        )
    };
    [
        valid(ROW),
        valid(OTHER),
        starts_before_end(ROW, OTHER),
        starts_before_end(OTHER, ROW),
    ]
}
