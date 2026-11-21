//! What a zero divisor yields: NULL (E6-4.6), or the runtime error `RDER-01` inside a derived
//! metric with `on_division_by_zero: error` (E6-4.6a, DER-3.1).

use menura_model::internal_model::{DivisionByZero, NormalizedName};

use super::ExpressionWriter;
use crate::values::text_sql;

/// The result of a division (`/`, `%` and `MOD`) whose divisor is 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZeroDivisor<'input> {
    /// `NULL`: every division outside derived metrics, and inside a derived metric with
    /// `on_division_by_zero: null` (E6-4.6).
    Null,
    /// The runtime error `RDER-01` naming the derived metric, by its normalized name, with
    /// `on_division_by_zero: error` (E6-4.6a).
    Error(&'input NormalizedName),
}

impl<'input> ZeroDivisor<'input> {
    /// The result inside the derived metric `metric` whose `on_division_by_zero` is `rule`.
    pub(crate) fn in_derived_metric(metric: &'input NormalizedName, rule: DivisionByZero) -> Self {
        match rule {
            DivisionByZero::Null => Self::Null,
            DivisionByZero::Error => Self::Error(metric),
        }
    }

    /// The SQL of the result. The error text is a literal, never a parameter (E6-4.11, EXT-1).
    pub(super) fn sql(self) -> String {
        match self {
            Self::Null => "NULL".to_owned(),
            Self::Error(metric) => {
                let message = format!("RDER-01: division by zero in metric {}", metric.as_str());
                format!("error({})", text_sql(&message))
            }
        }
    }
}

impl<'input> ExpressionWriter<'input, '_> {
    /// This writer, for the formula of the derived metric `metric` whose `on_division_by_zero`
    /// is `rule` (DER-3.1): its divisions (`/`, `%` and `MOD`) follow `rule`.
    pub(crate) fn in_derived_metric(
        mut self,
        metric: &'input NormalizedName,
        rule: DivisionByZero,
    ) -> Self {
        self.zero_divisor = ZeroDivisor::in_derived_metric(metric, rule);
        self
    }
}
