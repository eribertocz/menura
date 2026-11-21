//! The differential part of a case (CS-4.7): the rows of the compiled plan are compared under
//! CS-2.4 with those of an independent oracle, the hand-written reference SQL file that the case
//! names in `oracle`, next to the case, executed on the same fixture database.
//!
//! A divergence fails the case unless the reviewed file of divergences classifies it; a
//! classified divergence leaves the case pending, never passed, and an entry for a case that no
//! longer diverges fails it, so the file never keeps stale entries.

use menura_core::CompiledPlanDocument;

use crate::case::{Case, RowOrder};
use crate::divergences::{DIFFERENTIAL_FOLDER, DIVERGENCES_FILE, Divergence};
use crate::execution::{ExecutionFailure, execute_plan, execute_sql};
use crate::fixture::read_text;
use crate::query_part::QueryInputs;
use crate::report::CaseStatus;
use crate::row_comparison::{Comparison, rows_divergence};

/// Compares the rows of `plan` with those of the oracle of `case`, and applies its
/// classification when they diverge.
pub(crate) fn check_differential(
    case: &Case,
    plan: &CompiledPlanDocument,
    inputs: &QueryInputs<'_>,
) -> CaseStatus {
    let divergence = match divergence_from_oracle(case, plan, inputs) {
        Ok(divergence) => divergence,
        Err(reason) => return CaseStatus::Failed(reason),
    };
    classify(divergence, inputs.divergences.of(&case.id))
}

/// Why the rows of `plan` differ from the oracle's, or `None` when they match.
fn divergence_from_oracle(
    case: &Case,
    plan: &CompiledPlanDocument,
    inputs: &QueryInputs<'_>,
) -> Result<Option<String>, String> {
    let file_name = case
        .oracle
        .as_deref()
        .ok_or_else(|| "a differential case requires `oracle`".to_owned())?;
    let sql = read_text(&inputs.case_folder.join(file_name)).map_err(|error| error.to_string())?;
    let oracle = execute_sql(inputs.connection, &sql, &[]).map_err(|failure| {
        format!(
            "the oracle `{file_name}` fails on DuckDB: {}",
            describe(failure)
        )
    })?;
    let actual = execute_plan(inputs.connection, plan)
        .map_err(|failure| format!("the plan fails on DuckDB: {}", describe(failure)))?;
    let comparison = Comparison {
        row_order: case.expect.row_order.unwrap_or(RowOrder::Any),
        tolerance: case.expect.tolerance,
    };
    Ok(rows_divergence(
        &plan.result_schema,
        &actual,
        &oracle,
        comparison,
    ))
}

fn describe(failure: ExecutionFailure) -> String {
    match failure {
        ExecutionFailure::Parameter(reason) | ExecutionFailure::Engine(reason) => reason,
    }
}

/// The status of a differential part from its divergence and its classification.
fn classify(divergence: Option<String>, classified: Option<&Divergence>) -> CaseStatus {
    let file = format!("{DIFFERENTIAL_FOLDER}/{DIVERGENCES_FILE}");
    match (divergence, classified) {
        (None, None) => CaseStatus::Passed,
        (None, Some(_)) => CaseStatus::Failed(format!(
            "the rows match the oracle's, but `{file}` classifies a divergence: remove the entry"
        )),
        (Some(reason), None) => CaseStatus::Failed(format!(
            "the rows diverge from the oracle's and `{file}` does not classify it: {reason}"
        )),
        (Some(reason), Some(entry)) => CaseStatus::Pending(format!(
            "classified divergence from the oracle ({}: {}): {reason}",
            entry.classification.label(),
            entry.reason
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::CaseId;
    use crate::divergences::Classification;

    fn entry(classification: Classification) -> Divergence {
        Divergence {
            case: CaseId::new("cs-unit-001".to_owned()),
            classification,
            reason: "tracked by issue #1".to_owned(),
        }
    }

    #[test]
    fn cs_4_7_rows_that_match_the_oracle_pass() {
        assert_eq!(classify(None, None), CaseStatus::Passed);
    }

    #[test]
    fn cs_4_7_an_unclassified_divergence_fails() {
        let status = classify(Some("rows".to_owned()), None);
        let CaseStatus::Failed(reason) = status else {
            panic!("the case fails: {status:?}");
        };
        assert!(reason.contains("does not classify"), "{reason}");
    }

    #[test]
    fn cs_4_7_a_classified_divergence_is_pending_never_passed() {
        for classification in [
            Classification::Bug,
            Classification::KnownDeviation,
            Classification::Unsupported,
        ] {
            let status = classify(Some("rows".to_owned()), Some(&entry(classification)));
            let CaseStatus::Pending(reason) = status else {
                panic!("the case is pending: {status:?}");
            };
            assert!(reason.contains(classification.label()), "{reason}");
        }
    }

    #[test]
    fn cs_4_7_a_stale_classification_fails() {
        let status = classify(None, Some(&entry(Classification::Bug)));
        assert!(matches!(status, CaseStatus::Failed(_)), "{status:?}");
    }
}
