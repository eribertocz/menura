//! The runtime errors of semi-additive metrics (E6-4.6b, ADD-7).

use menura_model::internal_model::NormalizedName;

use crate::values::text_sql;

#[cfg(test)]
mod tests;

/// A runtime condition of a semi-additive metric that its rule turns into an error (ADD-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdditivityCondition {
    /// `RADD-01`: `missing: error` and an expected entity has no row.
    MissingEntity,
    /// `RADD-02`: `unexpected: error` and an unexpected row exists.
    UnexpectedRow,
    /// `RADD-03`: `nulls: error` and a NULL value exists.
    NullValue,
    /// `RADD-04`: `on_stale: error` and a carried value is too old.
    StaleValue,
}

impl AdditivityCondition {
    /// The code and condition text of ADD-7, backticks removed (E6-4.6b).
    fn code_and_condition(self) -> &'static str {
        match self {
            Self::MissingEntity => "RADD-01: missing: error and an expected entity has no row",
            Self::UnexpectedRow => "RADD-02: unexpected: error and an unexpected row exists",
            Self::NullValue => "RADD-03: nulls: error and a NULL value exists",
            Self::StaleValue => "RADD-04: on_stale: error and a carried value is too old",
        }
    }

    /// The SQL that raises this condition in the semi-additive metric `metric`, named by its
    /// normalized name. The text is a literal, never a parameter (E6-4.11, EXT-1).
    pub(crate) fn raise_sql(self, metric: &NormalizedName) -> String {
        let message = format!(
            "{} in metric {}",
            self.code_and_condition(),
            metric.as_str()
        );
        format!("error({})", text_sql(&message))
    }
}
