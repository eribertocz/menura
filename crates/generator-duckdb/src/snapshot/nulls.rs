//! What a NULL value of a semi-additive metric becomes (ADD-4.5, E6-4.6b).

use menura_model::internal_model::{NormalizedName, NullHandling};

use crate::additivity_error::AdditivityCondition;

/// `value` under `nulls` in the metric `metric`: `keep` leaves it, `zero` replaces a NULL by 0,
/// `error` raises `RADD-03`. `as_missing` is the caller's to apply, so it leaves the value.
pub(super) fn handle_null(value: &str, nulls: NullHandling, metric: &NormalizedName) -> String {
    match nulls {
        NullHandling::Keep | NullHandling::AsMissing => value.to_owned(),
        NullHandling::Zero => format!("COALESCE({value}, 0)"),
        NullHandling::Error => format!(
            "CASE WHEN {value} IS NULL THEN {} ELSE {value} END",
            AdditivityCondition::NullValue.raise_sql(metric)
        ),
    }
}
