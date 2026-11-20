//! Admission of a catalog state against a loaded model (CAT-9.1, ARC-5.2), including the
//! summary builds of CAT-8.8, which need the model's summaries (SUM-6.1.8).

use menura_contracts::types::catalog::{
    AggregateBuild, CatalogSchemaDocument, CatalogStateDocument, RelationData,
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

/// CAT-8.8: `aggregate_build` appears only on the relations of the model's summaries, and its
/// periods have the grain of the summary's coverage dimension.
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
        if let Some((period_pointer, period)) = period_of_another_grain(build, grain) {
            return Err(warning(
                RULE_VIOLATION,
                &format!("{pointer}{period_pointer}"),
                format!(
                    "period `{period}` of the summary build of `{}` is not of the grain of the \
                     summary's coverage dimension ({}); the state is ignored",
                    relation.source,
                    grain_name(grain)
                ),
            ));
        }
    }
    Ok(())
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

/// The first period of `build` whose grain is not `grain`, with its pointer inside the build.
fn period_of_another_grain(build: &AggregateBuild, grain: Option<Grain>) -> Option<(String, &str)> {
    let periods = [
        ("/coverage/from".to_owned(), build.coverage.from.as_str()),
        ("/coverage/to".to_owned(), build.coverage.to.as_str()),
    ]
    .into_iter()
    .chain(build.built_from.iter().enumerate().map(|(index, version)| {
        (
            format!("/built_from/{index}/period"),
            version.period.as_str(),
        )
    }))
    .chain(
        build
            .integrity_check
            .verified
            .iter()
            .enumerate()
            .map(|(index, version)| {
                (
                    format!("/integrity_check/verified/{index}/period"),
                    version.period.as_str(),
                )
            }),
    );
    periods
        .into_iter()
        .find(|(_, period)| grain.is_none() || period_grain(period) != grain)
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
