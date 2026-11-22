//! The `population` check with `mode: validity` (CHK-6.3, POP-5): the rows whose interval is
//! inverted, and each row whose interval overlaps another row of the same entity.

use menura_representations::logical_plan::PlanNode;
use menura_representations::names::ColumnName;

use super::shapes::ValidityCheck;
use super::sql::{
    aliased, all_equal, all_not_null, column, column_names, expect_names, known_columns,
    synthesized, violation,
};
use crate::body::{SelectBody, union_all};
use crate::error::GenerateError;
use crate::population::membership::column_type;
use crate::steps::{StepWriter, pass_through};

/// The alias of a row that overlaps another.
const ROW: &str = "\"menura_row\"";
/// The alias of the other row.
const OTHER: &str = "\"menura_other\"";

/// The synthesized columns of the `days` step.
struct DayColumns {
    from: String,
    to: String,
    row: String,
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
        let columns = DayColumns {
            from: synthesized("menura.from_day", &taken),
            to: synthesized("menura.to_day", &taken),
            row: synthesized("menura.row", &taken),
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
                "WHERE {from} IS NOT NULL AND {to} IS NOT NULL AND {from} > {to}"
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

    /// The step of every row with its `valid_from` day, its `valid_to` day and its row number.
    /// The step is read three times and may be computed each time, so the rows are numbered
    /// in the order of every scanned column: only equal rows, which are interchangeable, can
    /// swap numbers between two readings.
    fn days_step(
        &mut self,
        rows: &'input PlanNode,
        check: &ValidityCheck<'input>,
        columns: &DayColumns,
        written: &str,
    ) -> Result<String, GenerateError> {
        let mut items = pass_through(rows, rows.schema())?;
        for (field, name) in [
            (&check.valid_from, &columns.from),
            (&check.valid_to, &columns.to),
        ] {
            let logical_type = column_type(rows, rows, field)?;
            let day = self.day_sql(rows, rows, &column(None, field.as_str()), logical_type)?;
            items.push(aliased(&day, name));
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
        Ok(self.push_step("days", body))
    }
}

/// The row numbers of the rows that overlap another row of the same entity: both have a
/// whole entity and a `valid_from` day, neither is inverted, and each starts on or before the
/// day the other ends (a NULL `valid_to` is open-ended).
fn overlaps_body(check: &ValidityCheck<'_>, columns: &DayColumns, days: &str) -> SelectBody {
    let entity = |alias: &str| -> Vec<String> {
        check
            .entity
            .iter()
            .map(|name| column(Some(alias), name.as_str()))
            .collect()
    };
    let from = |alias: &str| column(Some(alias), &columns.from);
    let to = |alias: &str| column(Some(alias), &columns.to);
    let valid = |alias: &str| {
        format!(
            "{} IS NOT NULL AND ({} IS NULL OR {} <= {})",
            from(alias),
            to(alias),
            from(alias),
            to(alias)
        )
    };
    let mut conditions = vec![
        format!(
            "{} <> {}",
            column(Some(ROW), &columns.row),
            column(Some(OTHER), &columns.row)
        ),
        all_not_null(&entity(ROW)),
        all_equal(&entity(ROW), &entity(OTHER)),
        valid(ROW),
        valid(OTHER),
    ];
    conditions.push(format!(
        "({} IS NULL OR {} <= {})",
        to(OTHER),
        from(ROW),
        to(OTHER)
    ));
    conditions.push(format!(
        "({} IS NULL OR {} <= {})",
        to(ROW),
        from(OTHER),
        to(ROW)
    ));
    SelectBody {
        items: vec![aliased(&column(Some(ROW), &columns.row), &columns.row)],
        from: format!("{days} AS {ROW}"),
        clauses: vec![
            format!("JOIN {days} AS {OTHER} ON {}", conditions.join(" AND ")),
            "GROUP BY 1".to_owned(),
        ],
    }
}
