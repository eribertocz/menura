//! Rows 10 to 12a of E2-5: a query that names an element marked unusable in E0 fails with the
//! element's query error code (E0-4.1). Each row is checked over every named element before the
//! next row (E2-5.3).

use menura_model::internal_model::{DiagnosticCode, InternalModel};

use crate::error::{BindCode, BindDiagnostic};
use crate::model_elements::query_errors;
use crate::named::NamedElement;

/// Rows 10, 11 and 12: the metric rules of E0G-2, by the code each records.
const METRIC_RULE_ROWS: [(&str, BindCode); 3] = [
    (
        "E_UNAGGREGATED_FINER_GRAIN_REFERENCE",
        BindCode::UnaggregatedFinerGrainReference,
    ),
    (
        "E_DEFERRED_WINDOWED_METRIC",
        BindCode::DeferredWindowedMetric,
    ),
    ("E_AMBIGUOUS_MEASURE_GRAIN", BindCode::AmbiguousMeasureGrain),
];

/// Rows 10 to 12a, in order, over `named`.
pub(crate) fn check_rows_10_to_12a(
    model: &InternalModel,
    named: &[NamedElement],
) -> Result<(), BindDiagnostic> {
    for (code, bind_code) in METRIC_RULE_ROWS {
        let found = named.iter().find(|element| {
            query_errors(model, &element.target)
                .iter()
                .any(|recorded| recorded.as_str() == code)
        });
        if let Some(element) = found {
            return Err(unusable(element, bind_code, code));
        }
    }
    for element in named {
        let first = query_errors(model, &element.target)
            .into_iter()
            .filter_map(|code| row_12a_rank(code).map(|rank| (rank, code)))
            .min();
        if let Some((_, code)) = first {
            return Err(unusable(
                element,
                BindCode::Unusable(code.clone()),
                code.as_str(),
            ));
        }
    }
    Ok(())
}

/// The place of `code` in the order of row 12a: `E0B-08`, `E0B-09`, `E0D-03`–`E0D-07`,
/// `E0F-*`, `E0G-01`, `E0G-02`, `E0C-37`, `E0H-01`, then by code within a group; `None` when
/// row 12a does not report it.
fn row_12a_rank(code: &DiagnosticCode) -> Option<(u8, &str)> {
    let text = code.as_str();
    let group = match text {
        "E0B-08" => 0,
        "E0B-09" => 1,
        "E0D-03" | "E0D-04" | "E0D-05" | "E0D-06" | "E0D-07" => 2,
        _ if text.starts_with("E0F-") => 3,
        "E0G-01" => 4,
        "E0G-02" => 5,
        "E0C-37" => 6,
        "E0H-01" => 7,
        _ => return None,
    };
    Some((group, text))
}

/// The diagnostic of `element`, unusable with `code`.
fn unusable(element: &NamedElement, bind_code: BindCode, code: &str) -> BindDiagnostic {
    BindDiagnostic::new(
        bind_code,
        format!(
            "`{}` cannot be used: the model marks it unusable ({code}); see the load warnings \
             of the model",
            element.written
        ),
        &element.origin,
    )
}
