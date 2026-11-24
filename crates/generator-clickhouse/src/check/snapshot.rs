//! The `semi_additive` check of a metric (CHK-6.5, ADD-6): duplicate snapshots, and with a
//! time `over` the gaps between the first and the last period with data, the expected
//! entities without a row and the rows of unexpected entities.

use menura_contracts::types::common::Grain;

use super::shapes::{SnapshotCheck, SnapshotTime};
use super::sql::{
    aliased, all_equal, all_not_null, column, expect_names, known_columns, ordinals, synthesized,
    violation,
};
use crate::body::{SelectBody, union_all};
use crate::day::period_series;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::population::membership::column_type;
use crate::population::{PERIOD, ROW};
use crate::steps::{StepWriter, error, pass_through};
use crate::types::nullable_type;

/// The alias of an expected pair.
const EXPECTED: &str = "\"menura_expected\"";
/// The alias of a pair that has a row.
const SEEN: &str = "\"menura_seen\"";

/// The steps every condition reads: the snapshots (entity and period of each row with a whole
/// entity and an `over`) and the names of their columns.
struct Snapshots {
    /// The step of the snapshots.
    step: String,
    /// The quoted entity columns, in declared order.
    entity: Vec<String>,
    /// The name of the period column.
    period: String,
}

impl<'input> StepWriter<'input> {
    /// The union of the ADD-6 conditions of `check`, each row with its `violation` (CHK-6.5).
    pub(super) fn snapshot_check(
        &mut self,
        check: &SnapshotCheck<'input>,
    ) -> Result<SelectBody, GenerateError> {
        let rows = check.rows;
        expect_names(&check.result_columns, check.entity.len() + 2, rows)?;
        let snapshots = self.snapshots(check)?;
        let duplicate = duplicate_body(check, &snapshots);
        let Some(time) = &check.time else {
            return Ok(duplicate);
        };
        let seen = self.push_step(
            "seen",
            SelectBody {
                items: pair_items(None, &snapshots),
                from: snapshots.step.clone(),
                clauses: vec![format!("GROUP BY {}", ordinals(snapshots.entity.len() + 1))],
            },
        );
        let series = self.series(time.grain, &snapshots);
        let expected = self.expected(time, &snapshots, &series)?;
        let gap = gap_body(check, &snapshots, &series, &seen)?;
        let missing = absent_body(
            check,
            &snapshots,
            (&expected, EXPECTED),
            (&seen, SEEN),
            "missing_entity",
        );
        let unexpected = absent_body(
            check,
            &snapshots,
            (&seen, SEEN),
            (&expected, EXPECTED),
            "unexpected_entity",
        );
        Ok(union_all(duplicate, &[gap, missing, unexpected]))
    }

    /// The step of the rows with a whole entity and an `over`, each with its period: the
    /// `over` value truncated to `snapshot_grain` (E6-7.6, POP-3.5), or the `over` value
    /// itself when it is not a time field.
    fn snapshots(&mut self, check: &SnapshotCheck<'input>) -> Result<Snapshots, GenerateError> {
        let rows = check.rows;
        let written = self.write(rows)?;
        self.parameters.start_node(rows.origin());
        let entity = known_columns(rows, &check.entity.iter().collect::<Vec<_>>())?;
        let over = column(None, check.over.as_str());
        let value = match &check.time {
            Some(time) => self.period_start_sql(rows, &check.over, time.grain)?,
            None => over.clone(),
        };
        let period = synthesized("menura.period", &check.entity.iter().collect::<Vec<_>>());
        let mut present = entity.clone();
        present.push(over);
        let mut items: Vec<String> = entity.clone();
        items.push(aliased(&value, &period));
        let step = self.push_step(
            "snapshots",
            SelectBody {
                items,
                from: written.name,
                clauses: vec![format!("WHERE {}", all_not_null(&present))],
            },
        );
        Ok(Snapshots {
            step,
            entity,
            period,
        })
    }

    /// The step of every period at `grain` from the first to the last period of the
    /// snapshots, both included; no period when there is no snapshot.
    fn series(&mut self, grain: Grain, snapshots: &Snapshots) -> String {
        let first = "\"menura.first\"";
        let last = "\"menura.last\"";
        let start = "\"menura.start\"";
        let period = column(None, &snapshots.period);
        let bounds = self.push_step(
            "bounds",
            SelectBody {
                items: vec![
                    format!("minOrNull({period}) AS {first}"),
                    format!("maxOrNull({period}) AS {last}"),
                ],
                from: snapshots.step.clone(),
                clauses: Vec::new(),
            },
        );
        let starts = self.push_step(
            "starts",
            SelectBody {
                items: vec![format!(
                    "arrayJoin({}) AS {start}",
                    period_series(grain, first, last)
                )],
                from: bounds,
                clauses: Vec::new(),
            },
        );
        self.push_step(
            "series",
            SelectBody {
                items: vec![aliased(start, &snapshots.period)],
                from: starts,
                clauses: Vec::new(),
            },
        )
    }

    /// The step of the (entity, period) pairs the population expects in the periods of
    /// `series` (POP-3): its rows where `required_when` holds (POP-3.3) with a whole entity
    /// (POP-3.7), each paired with the periods its membership covers, each pair once.
    fn expected(
        &mut self,
        time: &SnapshotTime<'input>,
        snapshots: &Snapshots,
        series: &str,
    ) -> Result<String, GenerateError> {
        let population = time.population;
        if time.rule.entity.len() != snapshots.entity.len() {
            return Err(error(InvariantViolation::UnknownColumn, population));
        }
        let rows = self.required_rows(time)?;
        let period = column(Some(PERIOD), &snapshots.period);
        let covered = self.covers(
            population,
            population,
            &time.rule.membership,
            &period,
            time.grain,
        )?;
        let entity: Vec<String> = time
            .rule
            .entity
            .iter()
            .map(|name| column(Some(ROW), name.as_str()))
            .collect();
        let mut items: Vec<String> = entity
            .iter()
            .zip(&time.rule.entity)
            .zip(&snapshots.entity)
            .map(|((value, _), name)| format!("{value} AS {name}"))
            .collect();
        items.push(aliased(&period, &snapshots.period));
        Ok(self.push_step(
            "expected",
            SelectBody {
                items,
                from: format!("{rows} AS {ROW}"),
                clauses: vec![
                    format!("CROSS JOIN {series} AS {PERIOD}"),
                    format!("WHERE ({covered}) AND {}", all_not_null(&entity)),
                    format!("GROUP BY {}", ordinals(snapshots.entity.len() + 1)),
                ],
            },
        ))
    }
}

impl<'input> StepWriter<'input> {
    /// The step of the population's rows where `required_when` holds (POP-3.3), or of all its
    /// rows without one.
    fn required_rows(&mut self, time: &SnapshotTime<'input>) -> Result<String, GenerateError> {
        let population = time.population;
        let written = self.write(population)?;
        self.parameters.start_node(population.origin());
        let Some(condition) = &time.rule.required_when else {
            return Ok(written.name);
        };
        let condition =
            self.expression(population, Columns::Input(population.schema()), condition)?;
        let required = SelectBody {
            items: pass_through(population, population.schema())?,
            from: written.name,
            clauses: vec![format!("WHERE {condition}")],
        };
        Ok(self.push_step("required", required))
    }
}

/// The entity columns of `alias` (or unqualified), then the period, each under its own name.
fn pair_items(alias: Option<&str>, snapshots: &Snapshots) -> Vec<String> {
    let mut items: Vec<String> = snapshots
        .entity
        .iter()
        .map(|name| match alias {
            Some(alias) => format!("{alias}.{name} AS {name}"),
            None => name.clone(),
        })
        .collect();
    items.push(aliased(
        &column(alias, &snapshots.period),
        &snapshots.period,
    ));
    items
}

/// The result items: `values` (the entity values, then the period) under the result names,
/// then `violation`.
fn result_items(check: &SnapshotCheck<'_>, values: &[String], name: &str) -> Vec<String> {
    let (fields, added) = check.result_columns.split_at(values.len());
    values
        .iter()
        .zip(fields)
        .map(|(value, field)| aliased(value, field))
        .chain(added.iter().map(|added| violation(name, added)))
        .collect()
}

/// `duplicate_snapshot`: an entity and period that more than one row has.
fn duplicate_body(check: &SnapshotCheck<'_>, snapshots: &Snapshots) -> SelectBody {
    let mut values = snapshots.entity.clone();
    values.push(column(None, &snapshots.period));
    SelectBody {
        items: result_items(check, &values, "duplicate_snapshot"),
        from: snapshots.step.clone(),
        clauses: vec![
            format!("GROUP BY {}", ordinals(snapshots.entity.len() + 1)),
            "HAVING count() > 1".to_owned(),
        ],
    }
}

/// `gap`: a period of the series that no row has, with NULL entity columns of the entity
/// types.
fn gap_body(
    check: &SnapshotCheck<'_>,
    snapshots: &Snapshots,
    series: &str,
    seen: &str,
) -> Result<SelectBody, GenerateError> {
    let rows = check.rows;
    let mut values = check
        .entity
        .iter()
        .map(|name| {
            let logical_type = column_type(rows, rows, name)?;
            let name = nullable_type(logical_type)
                .ok_or_else(|| error(InvariantViolation::UnsupportedType, rows))?;
            Ok(format!("CAST(NULL AS {name})"))
        })
        .collect::<Result<Vec<_>, GenerateError>>()?;
    let period = column(None, &snapshots.period);
    values.push(period.clone());
    Ok(SelectBody {
        items: result_items(check, &values, "gap"),
        from: series.to_owned(),
        clauses: vec![format!(
            "WHERE {period} NOT IN (SELECT {period} FROM {seen})"
        )],
    })
}

/// The pairs of `present` (step, alias) that `absent` (step, alias) does not have, under the
/// violation `name`: `missing_entity` (expected without a row) or `unexpected_entity` (a row
/// not expected, ADD-4.4).
fn absent_body(
    check: &SnapshotCheck<'_>,
    snapshots: &Snapshots,
    present: (&str, &str),
    absent: (&str, &str),
    name: &str,
) -> SelectBody {
    let qualified = |alias: &str| -> Vec<String> {
        snapshots
            .entity
            .iter()
            .map(|entity| format!("{alias}.{entity}"))
            .chain([column(Some(alias), &snapshots.period)])
            .collect()
    };
    let values = qualified(present.1);
    let on = all_equal(&values, &qualified(absent.1));
    SelectBody {
        items: result_items(check, &values, name),
        from: format!("{} AS {}", present.0, present.1),
        clauses: vec![
            format!("LEFT JOIN {} AS {} ON {on}", absent.0, absent.1),
            format!(
                "WHERE {} IS NULL",
                column(Some(absent.1), &snapshots.period)
            ),
        ],
    }
}
