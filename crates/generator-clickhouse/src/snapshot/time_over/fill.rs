//! The values of missing pairs (ADD-4.3, E4-7.5, E4-7.6): 0 from the `added` rows under
//! `missing: zero`, or the latest earlier value of the entity under `carry_forward`.

use menura_model::internal_model::{NullHandling, StaleHandling};
use menura_representations::logical_plan::{PlanNode, SnapshotMissing};

use super::frame::{Frame, PERIOD, STALE, equalities, qualified, truncated};
use crate::body::SelectBody;
use crate::day::periods_between;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::steps::{StepWriter, column_type, error};

/// The alias of a missing pair.
const MISSING: &str = "\"menura_missing\"";
/// The alias of a row that fills it.
const FILL: &str = "\"menura_fill\"";

impl<'input> StepWriter<'input> {
    /// The step of the values of the missing pairs, with the columns of the values; under
    /// `on_stale: error`, also `menura.stale`. `extra` is the step of the `added` or `carried`
    /// rows.
    pub(super) fn fill(
        &mut self,
        frame: &Frame<'input>,
        missing: &str,
        extra: &str,
    ) -> Result<String, GenerateError> {
        match &frame.time.missing {
            SnapshotMissing::Zero { added } => {
                let rows = self.added_rows(frame, added, extra)?;
                let rows = self.push_step("added", rows);
                let body = zero_values(frame, missing, &rows)?;
                Ok(self.push_step("zero", body))
            }
            SnapshotMissing::CarryForward {
                carried,
                max_carry_forward,
                on_stale,
            } => {
                let nulls = carried_nulls(frame.rule.nulls);
                let rows = self.period_values(frame, carried, extra, Some(nulls))?;
                let rows = self.push_step("carried", rows);
                let limit = max_carry_forward.get();
                let body = self.carry_values(frame, missing, &rows, limit, *on_stale)?;
                Ok(self.push_step("carry", body))
            }
            SnapshotMissing::Exclude | SnapshotMissing::Error => {
                Err(error(InvariantViolation::NodeOutsideSlice, frame.node))
            }
        }
    }

    /// The keys, the entity and the period of each `added` row. An added row's `over` is the
    /// first instant of its expected period (E4-7.6), so its period follows from `over` like
    /// that of any other row.
    fn added_rows(
        &mut self,
        frame: &Frame<'input>,
        added: &'input PlanNode,
        name: &str,
    ) -> Result<SelectBody, GenerateError> {
        let node = frame.node;
        let columns = Columns::Input(added.schema());
        let mut items = self.named_items(node, added, &frame.rule.keys)?;
        items.extend(self.named_items(node, added, &frame.rule.entity)?);
        let period = self.expression(node, columns, &frame.time.period)?;
        let period_type = self.writer(node, columns).type_of(&frame.time.period)?;
        let day = self.day_sql(node, added, &period, &period_type)?;
        items.push(format!("{day} AS {PERIOD}"));
        Ok(SelectBody {
            items,
            from: name.to_owned(),
            clauses: Vec::new(),
        })
    }

    /// The latest `carried` value of each missing pair's entity before its period, with the
    /// carried row's keys but the unit key of the pair's period (ADD-4.7). A value more than
    /// `limit` periods old is stale: dropped, or marked for `RADD-04`.
    fn carry_values(
        &mut self,
        frame: &Frame<'input>,
        missing: &str,
        carried: &str,
        limit: u64,
        on_stale: StaleHandling,
    ) -> Result<SelectBody, GenerateError> {
        let mut items = self.pair_keys(frame, MISSING)?;
        items.extend(qualified(FILL, &frame.entity));
        items.push(format!("{MISSING}.{PERIOD}"));
        items.extend(qualified(FILL, &frame.components));
        let age = periods_between(
            frame.time.grain,
            &format!("{FILL}.{PERIOD}"),
            &format!("{MISSING}.{PERIOD}"),
        );
        let entity = equalities(FILL, &frame.entity, MISSING, &frame.entity);
        let partition = qualified(MISSING, &frame.entity).join(", ");
        let rank = format!(
            "QUALIFY RANK() OVER (PARTITION BY {partition}, {MISSING}.{PERIOD} ORDER BY {FILL}.{PERIOD} DESC NULLS LAST) = 1"
        );
        let latest = match on_stale {
            StaleHandling::Exclude => format!("{rank} AND {age} <= {limit}"),
            StaleHandling::Error => {
                items.push(format!("{age} > {limit} AS {STALE}"));
                rank
            }
        };
        Ok(SelectBody {
            items,
            from: format!("{missing} AS {MISSING}"),
            clauses: vec![
                format!("JOIN {carried} AS {FILL} ON {entity}"),
                format!("WHERE {FILL}.{PERIOD} < {MISSING}.{PERIOD}"),
                latest,
            ],
        })
    }

    /// The keys of a carried row, `menura_fill`, except the unit key, which is that of the
    /// period of `pair`, the alias of the missing pair (ADD-4.7).
    fn pair_keys(
        &mut self,
        frame: &Frame<'input>,
        pair: &str,
    ) -> Result<Vec<String>, GenerateError> {
        let mut items = Vec::new();
        for (key, name) in frame.rule.keys.iter().zip(&frame.keys) {
            match &frame.time.unit {
                Some(unit) if unit.column == key.name => {
                    let start = truncated(&format!("{pair}.{PERIOD}"), unit.grain);
                    let value = self.unit_value(frame, &start, key.name.as_str())?;
                    items.push(format!("{value} AS {name}"));
                }
                _ => items.push(format!("{FILL}.{name}")),
            }
        }
        Ok(items)
    }

    /// `start`, a `date`, as a value of the key column `key` of the node.
    fn unit_value(
        &mut self,
        frame: &Frame<'input>,
        start: &str,
        key: &str,
    ) -> Result<String, GenerateError> {
        let node = frame.node;
        let logical_type = node
            .schema()
            .columns()
            .iter()
            .find(|column| column.name.as_str() == key)
            .map(|column| column.logical_type.clone())
            .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
        self.start_sql(node, start, &logical_type)
    }
}

/// The value 0 of each component for the `added` rows of the missing pairs (E4-7.6). An
/// added row's `over` is the first instant of its period, so its keys, the unit key
/// included, are those of that period.
fn zero_values(frame: &Frame<'_>, missing: &str, added: &str) -> Result<SelectBody, GenerateError> {
    let mut items = qualified(FILL, &frame.keys);
    items.extend(qualified(FILL, &frame.entity));
    items.push(format!("{FILL}.{PERIOD}"));
    for component in &frame.rule.components {
        let result_type = column_type(frame.node, component.name.as_str())?;
        items.push(format!(
            "CAST(0 AS {result_type}) AS {}",
            crate::identifiers::quote(component.name.as_str())
        ));
    }
    let entity = equalities(FILL, &frame.entity, MISSING, &frame.entity);
    Ok(SelectBody {
        items,
        from: format!("{added} AS {FILL}"),
        clauses: vec![format!(
            "JOIN {missing} AS {MISSING} ON {entity} AND {FILL}.{PERIOD} = {MISSING}.{PERIOD}"
        )],
    })
}

/// `nulls` for the carried read: a carried NULL is carried as NULL under `error`, since only
/// the conditions inside the queried range count (E6-4.6b) and the carried read reaches before
/// it (E4-7.5).
fn carried_nulls(nulls: NullHandling) -> NullHandling {
    match nulls {
        NullHandling::Error => NullHandling::Keep,
        other => other,
    }
}
