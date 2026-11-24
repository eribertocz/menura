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
        let from = self.parameters.bind_slot(Binding::PeriodFrom);
        let to = self.parameters.bind_slot(Binding::PeriodTo);
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
    let year = year(period);
    format!(
        "CASE \
         WHEN length({period}) = 4 THEN makeDate32({year}, 1, 1) \
         WHEN substring({period}, 6, 1) = 'Q' THEN \
         makeDate32({year}, toInt32(substring({period}, 7, 1)) * 3 - 2, 1) \
         WHEN substring({period}, 6, 1) = 'W' THEN {} \
         WHEN length({period}) = 7 THEN \
         makeDate32({year}, toInt32(substring({period}, 6, 2)), 1) \
         ELSE toDate32({period}) END",
        week_start(period)
    )
}

/// The last day of the period `period`, read from its shape as [`first_day`] reads it.
fn last_day(period: &str) -> String {
    let year = year(period);
    format!(
        "CASE \
         WHEN length({period}) = 4 THEN makeDate32({year}, 12, 31) \
         WHEN substring({period}, 6, 1) = 'Q' THEN addDays(addMonths(\
         makeDate32({year}, toInt32(substring({period}, 7, 1)) * 3 - 2, 1), 3), -1) \
         WHEN substring({period}, 6, 1) = 'W' THEN addDays({}, 6) \
         WHEN length({period}) = 7 THEN \
         toLastDayOfMonth(makeDate32({year}, toInt32(substring({period}, 6, 2)), 1)) \
         ELSE toDate32({period}) END",
        week_start(period)
    )
}

/// The year of `period`, its first four characters.
fn year(period: &str) -> String {
    format!("toInt32(substring({period}, 1, 4))")
}

/// The Monday of the ISO week `YYYY-Www`: the week of 4 January is week 1.
fn week_start(period: &str) -> String {
    format!(
        "addDays(toMonday(makeDate32({}, 1, 4)), 7 * (toInt32(substring({period}, 7, 2)) - 1))",
        year(period)
    )
}
