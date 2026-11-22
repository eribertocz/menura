//! The step of a `PeriodRange` node (CHK-7.4): the rows whose coverage day lies in the range of
//! the bound periods, each bound read in its canonical form (CAT-5.1) from its shape.

use menura_contracts::types::descriptions::Binding;
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::{StepWriter, error, pass_through};

impl<'input> StepWriter<'input> {
    /// The step of `input`, then the step that keeps its rows whose day of `coverage`
    /// (POP-3.5) is on or after the first day of `period_from` and on or before the last day
    /// of `period_to`. A NULL day compares to unknown, so its row is not kept.
    pub(crate) fn period_range(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        coverage: &ColumnName,
    ) -> Result<SelectBody, GenerateError> {
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let column = input
            .schema()
            .columns()
            .iter()
            .find(|column| &column.name == coverage)
            .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
        let day = self.day_sql(node, input, &quote(coverage.as_str()), &column.logical_type)?;
        // The day is written first, so its time-zone slot is bound first (E6-4.2).
        let from = text(&self.parameters.bind_slot(Binding::PeriodFrom));
        let to = text(&self.parameters.bind_slot(Binding::PeriodTo));
        Ok(SelectBody {
            items: pass_through(node, input.schema())?,
            from: written.name,
            clauses: vec![format!(
                "WHERE {day} >= {} AND {day} <= {}",
                first_day(&from),
                last_day(&to)
            )],
        })
    }
}

/// The first day of the period `period`, a canonical period of any grain (CAT-5.1), read from
/// its shape: `YYYY`, `YYYY-Qn`, `YYYY-Www` (ISO 8601 week), `YYYY-MM` or `YYYY-MM-DD`.
fn first_day(period: &str) -> String {
    let year = format!("CAST(substr({period}, 1, 4) AS INTEGER)");
    format!(
        "CASE \
         WHEN length({period}) = 4 THEN make_date({year}, 1, 1) \
         WHEN substr({period}, 6, 1) = 'Q' THEN \
         make_date({year}, CAST(substr({period}, 7, 1) AS INTEGER) * 3 - 2, 1) \
         WHEN substr({period}, 6, 1) = 'W' THEN \
         CAST(date_trunc('week', make_date({year}, 1, 4)) AS DATE) \
         + 7 * (CAST(substr({period}, 7, 2) AS INTEGER) - 1) \
         WHEN length({period}) = 7 THEN \
         make_date({year}, CAST(substr({period}, 6, 2) AS INTEGER), 1) \
         ELSE CAST({period} AS DATE) END"
    )
}

/// The last day of the period `period`: its first day, plus the length of a period of its
/// grain, minus one day.
fn last_day(period: &str) -> String {
    format!(
        "CAST({} + CASE \
         WHEN length({period}) = 4 THEN INTERVAL 1 YEAR \
         WHEN substr({period}, 6, 1) = 'Q' THEN INTERVAL 3 MONTH \
         WHEN substr({period}, 6, 1) = 'W' THEN INTERVAL 7 DAY \
         WHEN length({period}) = 7 THEN INTERVAL 1 MONTH \
         ELSE INTERVAL 1 DAY END - INTERVAL 1 DAY AS DATE)",
        first_day(period)
    )
}

/// The slot `placeholder` as text: `CAST($n AS VARCHAR)`.
fn text(placeholder: &str) -> String {
    format!("CAST({placeholder} AS VARCHAR)")
}
