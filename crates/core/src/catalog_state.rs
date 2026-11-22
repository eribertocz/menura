//! Admission of a catalog state against a loaded model (CAT-9.1, ARC-5.2), including the
//! summary builds of CAT-8.8, which need the model's summaries (SUM-6.1.8).

use menura_contracts::types::catalog::{
    AggregateBuild, CatalogSchemaDocument, CatalogStateDocument, PeriodVersion, RelationData,
};
use menura_contracts::types::common::Grain;
use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_contracts::{admit_catalog_state, state_former_name_warning};
use menura_model::internal_model::{InternalModel, Summary};

/// The code of a state that is not valid UTF-8 JSON (CAT-9.1).
const NOT_JSON: &str = "WCAT-01";
/// The code of a state that breaks a CAT-8 rule.
const RULE_VIOLATION: &str = "WCAT-03";

/// What admitting a catalog state gave: the state, or none when it is ignored, and its warning
/// (ARC-5.2, CON-0.5).
pub(crate) struct StateAdmission {
    /// The admitted state, or `None` when it is ignored.
    pub(crate) state: Option<CatalogStateDocument>,
    /// The warning of an ignored state (`WCAT-01` to `WCAT-04`), or the `WCAT-05` of an
    /// admitted state with the former contract name.
    pub(crate) warning: Option<Diagnostic>,
}

/// Admits the catalog state `bytes` against the loaded `model` and its `schema`. A state that
/// is not admitted is ignored with the warning of its first failure, in the order of CAT-9.1,
/// with CAT-8.8 last. An admitted state with the former contract name keeps `WCAT-05`.
pub(crate) fn admit(
    bytes: &[u8],
    schema: &CatalogSchemaDocument,
    model: &InternalModel,
) -> StateAdmission {
    match admitted_state(bytes, schema, model) {
        Ok(state) => StateAdmission {
            warning: state_former_name_warning(&state).map(|warning| warning.diagnostic()),
            state: Some(state),
        },
        Err(warning) => StateAdmission {
            state: None,
            warning: Some(warning),
        },
    }
}

/// The admitted state, or the warning of its first failure.
fn admitted_state(
    bytes: &[u8],
    schema: &CatalogSchemaDocument,
    model: &InternalModel,
) -> Result<CatalogStateDocument, Diagnostic> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Err(warning(
            NOT_JSON,
            "",
            "the catalog state is not valid UTF-8, so it is not JSON; it is ignored".to_owned(),
        ));
    };
    let state = admit_catalog_state(text, schema).map_err(|error| error.diagnostic())?;
    check_summary_builds(&state, model)?;
    Ok(state)
}

/// CAT-8.8: `aggregate_build` appears only on the relations of the model's summaries, its
/// coverage has the grain of the summary's coverage dimension, and its build and check periods
/// have a grain from which that grain derives by exact grouping.
fn check_summary_builds(
    state: &CatalogStateDocument,
    model: &InternalModel,
) -> Result<(), Diagnostic> {
    for (index, relation) in state.relations.iter().enumerate() {
        let RelationData::AggregateBuild(build) = &relation.data else {
            continue;
        };
        let pointer = format!("/relations/{index}/aggregate_build");
        let Some(summary) = summary_with_source(model, &relation.source) else {
            return Err(warning(
                RULE_VIOLATION,
                &pointer,
                format!(
                    "relation `{}` has an `aggregate_build`, but the model declares no summary \
                     with that source; the state is ignored",
                    relation.source
                ),
            ));
        };
        let grain = coverage_grain(summary);
        if let Some(period) = misplaced_period(build, grain) {
            return Err(warning(
                RULE_VIOLATION,
                &format!("{pointer}{}", period.pointer),
                format!(
                    "period `{}` of the summary build of `{}` is not of {}; the state is ignored",
                    period.period,
                    relation.source,
                    period.expected.describe(grain)
                ),
            ));
        }
    }
    Ok(())
}

/// The grain a period of a summary build must have.
#[derive(Clone, Copy)]
enum ExpectedGrain {
    /// A `coverage` period: the grain of the summary's coverage dimension.
    Coverage,
    /// A `built_from` or `verified` period: a grain from which that grain derives.
    Finer,
}

impl ExpectedGrain {
    /// Whether a period of `period_grain` is accepted for a summary of `summary_grain`. A
    /// coverage dimension without a grain accepts no period.
    fn accepts(self, period_grain: Option<Grain>, summary_grain: Option<Grain>) -> bool {
        match (self, period_grain, summary_grain) {
            (_, Some(_), None) | (_, None, _) => false,
            (Self::Coverage, Some(period), Some(summary)) => period == summary,
            (Self::Finer, Some(period), Some(summary)) => derives_from(summary, period),
        }
    }

    /// The expected grain, in the words of a warning message.
    fn describe(self, summary_grain: Option<Grain>) -> String {
        let name = grain_name(summary_grain);
        match self {
            Self::Coverage => format!("the grain of the summary's coverage dimension ({name})"),
            Self::Finer => format!(
                "a grain from which the grain of the summary's coverage dimension ({name}) \
                 derives"
            ),
        }
    }
}

/// A period of a summary build, with its pointer inside the build and the grain it must have.
struct BuildPeriod<'build> {
    pointer: String,
    period: &'build str,
    expected: ExpectedGrain,
}

/// Whether `coarse` derives from `fine` by exact grouping (SUM-3.4, CAT-7.2 condition 1).
fn derives_from(coarse: Grain, fine: Grain) -> bool {
    match fine {
        Grain::Day => true,
        Grain::Week => coarse == Grain::Week,
        Grain::Month => matches!(coarse, Grain::Month | Grain::Quarter | Grain::Year),
        Grain::Quarter => matches!(coarse, Grain::Quarter | Grain::Year),
        Grain::Year => coarse == Grain::Year,
    }
}

/// The summary of the model whose dataset has `source`.
fn summary_with_source<'model>(
    model: &'model InternalModel,
    source: &str,
) -> Option<&'model Summary> {
    model.parts().summaries.iter().find(|summary| {
        model
            .dataset(&summary.dataset)
            .is_some_and(|dataset| dataset.source == source)
    })
}

/// The grain of the coverage dimension of `summary`. A coverage dimension without a grain
/// matches no period.
fn coverage_grain(summary: &Summary) -> Option<Grain> {
    summary
        .dimensions
        .iter()
        .find(|dimension| dimension.field == summary.coverage_field)
        .and_then(|dimension| dimension.grain)
}

/// The first period of `build`, in document order, whose grain is not accepted for a summary
/// of `grain`.
fn misplaced_period(build: &AggregateBuild, grain: Option<Grain>) -> Option<BuildPeriod<'_>> {
    let coverage = [
        ("/coverage/from", &build.coverage.from),
        ("/coverage/to", &build.coverage.to),
    ]
    .into_iter()
    .map(|(pointer, period)| BuildPeriod {
        pointer: pointer.to_owned(),
        period: period.as_str(),
        expected: ExpectedGrain::Coverage,
    });
    let built_from = listed_periods("/built_from", &build.built_from);
    let verified = listed_periods("/integrity_check/verified", &build.integrity_check.verified);
    coverage
        .chain(built_from)
        .chain(verified)
        .find(|period| !period.expected.accepts(period_grain(period.period), grain))
}

/// The periods of a `built_from` or `verified` list at `list`.
fn listed_periods<'build>(
    list: &'build str,
    versions: &'build [PeriodVersion],
) -> impl Iterator<Item = BuildPeriod<'build>> {
    versions
        .iter()
        .enumerate()
        .map(move |(index, version)| BuildPeriod {
            pointer: format!("{list}/{index}/period"),
            period: version.period.as_str(),
            expected: ExpectedGrain::Finer,
        })
}

/// The grain a canonical period's shape denotes (CAT-5.1, CAT-8.6). The state is admitted, so
/// every period has one of the canonical shapes.
fn period_grain(period: &str) -> Option<Grain> {
    match period.len() {
        4 => Some(Grain::Year),
        7 if period.as_bytes().get(5) == Some(&b'Q') => Some(Grain::Quarter),
        7 => Some(Grain::Month),
        8 if period.as_bytes().get(5) == Some(&b'W') => Some(Grain::Week),
        10 => Some(Grain::Day),
        _ => None,
    }
}

fn grain_name(grain: Option<Grain>) -> &'static str {
    match grain {
        Some(Grain::Day) => "day",
        Some(Grain::Week) => "week",
        Some(Grain::Month) => "month",
        Some(Grain::Quarter) => "quarter",
        Some(Grain::Year) => "year",
        None => "no grain",
    }
}

fn warning(code: &str, pointer: &str, message: String) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        message,
        location: Some(DiagnosticLocation {
            document: InputDocument::CatalogState,
            pointer: pointer.to_owned(),
            line: None,
            column: None,
        }),
    }
}
