//! The `population` check with `mode: periodic` (CHK-6.4, POP-5): the pairs of entity and
//! period that more than one row has.

use menura_contracts::types::common::Grain;
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::ColumnName;

use super::shapes::PeriodicCheck;
use super::sql::{
    aliased, all_not_null, column, column_names, expect_names, known_columns, ordinals, synthesized,
};
use crate::body::SelectBody;
use crate::day::grain_name;
use crate::error::GenerateError;
use crate::population::membership::column_type;
use crate::steps::{StepWriter, pass_through};

impl<'input> StepWriter<'input> {
    /// The groups of rows with a whole entity and the same non-NULL `period` truncated to
    /// `grain` (POP-3.6) that have more than one row, with the first day of the period as a
    /// `date` and the number of rows as a 64-bit signed integer (CHK-6.4). The NULL test reads
    /// `period` itself, whose period is NULL exactly when it is.
    pub(super) fn periodic_check(
        &mut self,
        check: &PeriodicCheck<'input>,
    ) -> Result<SelectBody, GenerateError> {
        let rows = check.rows;
        let entity_count = check.entity.len();
        expect_names(&check.result_columns, entity_count + 2, rows)?;
        let period_name = synthesized("menura.period", &column_names(rows));
        let periods = self.periods_step(check, &period_name)?;
        let entity = known_columns(rows, &check.entity.iter().collect::<Vec<_>>())?;
        let mut values = entity.clone();
        values.push(column(None, &period_name));
        values.push("CAST(COUNT(*) AS BIGINT)".to_owned());
        let mut present = entity;
        present.push(column(None, check.period.as_str()));
        Ok(SelectBody {
            items: values
                .iter()
                .zip(&check.result_columns)
                .map(|(value, name)| aliased(value, name))
                .collect(),
            from: periods,
            clauses: vec![
                format!("WHERE {}", all_not_null(&present)),
                format!("GROUP BY {}", ordinals(entity_count + 1)),
                "HAVING COUNT(*) > 1".to_owned(),
            ],
        })
    }

    /// The step of every row with its period under `period_name`.
    fn periods_step(
        &mut self,
        check: &PeriodicCheck<'input>,
        period_name: &str,
    ) -> Result<String, GenerateError> {
        let rows = check.rows;
        let written = self.write(rows)?;
        self.parameters.start_node(rows.origin());
        let period = self.period_start_sql(rows, &check.period, check.grain)?;
        let mut items = pass_through(rows, rows.schema())?;
        items.push(aliased(&period, period_name));
        Ok(self.push_step(
            "periods",
            SelectBody {
                items,
                from: written.name,
                clauses: Vec::new(),
            },
        ))
    }

    /// The first day of the period at `grain` of the column `name` of `rows`, as a `date`:
    /// its day (POP-3.5) truncated to `grain` (E6-5.2).
    pub(super) fn period_start_sql(
        &mut self,
        rows: &'input PlanNode,
        name: &ColumnName,
        grain: Grain,
    ) -> Result<String, GenerateError> {
        let logical_type = column_type(rows, rows, name)?;
        let day = self.day_sql(rows, rows, &column(None, name.as_str()), logical_type)?;
        Ok(format!(
            "CAST(date_trunc('{}', {day}) AS DATE)",
            grain_name(grain)
        ))
    }
}
