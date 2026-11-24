//! The values of a time `Snapshot` per (result group, entity, period) (E4-7.1), and the same
//! values after the `unexpected` and `nulls` rules (ADD-4.4, ADD-4.5).

use menura_model::internal_model::NullHandling;
use menura_representations::logical_plan::PlanNode;

use super::frame::{EXPECTED, Frame, PERIOD, equalities, qualified};
use super::reads_unexpected;
use crate::body::SelectBody;
use crate::error::GenerateError;
use crate::expression::Columns;
use crate::identifiers::quote;
use crate::snapshot::group_by;
use crate::snapshot::nulls::handle_null;
use crate::steps::StepWriter;

/// The alias of a row of the values.
pub(super) const VALUE: &str = "\"menura_value\"";

impl<'input> StepWriter<'input> {
    /// One row per (keys, entity, period) of `source`, written as `source_name`, with its
    /// components; under `nulls`, when given, their NULL values follow the rule and, for
    /// `as_missing`, a period with a NULL value is dropped.
    pub(super) fn period_values(
        &mut self,
        frame: &Frame<'input>,
        source: &'input PlanNode,
        source_name: &str,
        nulls: Option<NullHandling>,
    ) -> Result<SelectBody, GenerateError> {
        let node = frame.node;
        let columns = Columns::Input(source.schema());
        let mut items = self.named_items(node, source, &frame.rule.keys)?;
        items.extend(self.named_items(node, source, &frame.rule.entity)?);
        let period = self.expression(node, columns, &frame.time.period)?;
        let period_type = self.writer(node, columns).type_of(&frame.time.period)?;
        let period = self.day_sql(node, source, &period, &period_type)?;
        items.push(format!("{period} AS {PERIOD}"));
        let mut clauses = group_by(items.len());
        let mut known = Vec::new();
        for component in &frame.rule.components {
            let call = self
                .writer(node, columns)
                .aggregate(&component.call, component.filter.as_ref())?;
            let value = match nulls {
                Some(rule) => handle_null(&call, rule, &frame.rule.metric),
                None => call.clone(),
            };
            items.push(format!("{value} AS {}", quote(component.name.as_str())));
            known.push(format!("{call} IS NOT NULL"));
        }
        if nulls == Some(NullHandling::AsMissing) {
            clauses.push(format!("HAVING {}", known.join(" AND ")));
        }
        Ok(SelectBody {
            items,
            from: source_name.to_owned(),
            clauses,
        })
    }
}

/// The rows of `values` that `unexpected` keeps, their NULL values after `nulls`. Under
/// `error`, the rows stay as they are: the condition is raised by the reduction.
pub(super) fn checked(frame: &Frame<'_>, values: &str) -> SelectBody {
    let rule = frame.rule;
    let mut items = qualified(VALUE, &frame.keys);
    items.extend(qualified(VALUE, &frame.entity));
    items.push(format!("{VALUE}.{PERIOD}"));
    let nulls = match rule.nulls {
        NullHandling::Zero => NullHandling::Zero,
        NullHandling::Keep | NullHandling::AsMissing | NullHandling::Error => NullHandling::Keep,
    };
    let mut conditions = Vec::new();
    for component in &frame.components {
        let value = format!("{VALUE}.{component}");
        items.push(format!(
            "{} AS {component}",
            handle_null(&value, nulls, &rule.metric)
        ));
        if rule.nulls == NullHandling::AsMissing {
            conditions.push(format!("{value} IS NOT NULL"));
        }
    }
    let mut clauses = Vec::new();
    if reads_unexpected(frame.time) {
        clauses.push(expected_join(frame));
        if frame.time.unexpected == menura_model::internal_model::UnexpectedHandling::Exclude {
            conditions.insert(
                0,
                format!("{EXPECTED}.{} IS NOT NULL", frame.expected_period),
            );
        }
    }
    if !conditions.is_empty() {
        clauses.push(format!("WHERE {}", conditions.join(" AND ")));
    }
    SelectBody {
        items,
        from: format!("{values} AS {VALUE}"),
        clauses,
    }
}

/// `LEFT JOIN expected AS menura_expected ON` the entity and the period of the value.
pub(super) fn expected_join(frame: &Frame<'_>) -> String {
    let entity = equalities(VALUE, &frame.entity, EXPECTED, &frame.expected_entity);
    format!(
        "LEFT JOIN {} AS {EXPECTED} ON {entity} AND {VALUE}.{PERIOD} = {}",
        frame.expected, frame.expected_day
    )
}
