//! The summaries of the internal model and their definition hashes (E0H-2 "Summaries",
//! E0H-3.3).

use semantic_compiler_contracts::hash_value;
use semantic_compiler_contracts::types::common::Grain;
use semantic_compiler_model::internal_model::{
    Dataset, NormalizedName, Summary, SummaryDimension, SummaryMeasure,
};
use serde_json::Value as Json;

use super::documents::{field_definition, name, object, serialized, text, tree};
use super::index::ReadNames;
use super::located::{located, written};
use super::trees::parse;
use crate::expressions::ExpressionPlace;
use crate::extensions::summaries::{Grain as DeclaredGrain, SummaryDeclaration};

/// The summary `declaration`, with its definition hash computed over `datasets`.
pub(super) fn summary(
    declaration: &SummaryDeclaration,
    dataset: NormalizedName,
    datasets: &[Dataset],
) -> Summary {
    let mut summary = Summary {
        dataset,
        of: written(&declaration.of),
        dimensions: declaration
            .dimensions
            .iter()
            .map(|dimension| SummaryDimension {
                field: written(&dimension.field),
                base: written(&dimension.base),
                grain: dimension.grain.map(grain),
            })
            .collect(),
        measures: declaration
            .measures
            .iter()
            .map(|measure| SummaryMeasure {
                field: written(&measure.field),
                base_aggregate: parse(&measure.base_aggregate, ExpressionPlace::Metric).ok(),
            })
            .collect(),
        coverage_field: written(&declaration.coverage_field),
        definition_hash: hash_value(&Json::Null),
        location: located(&declaration.data, declaration.location),
    };
    summary.definition_hash = hash_value(&definition(&summary, datasets));
    summary
}

fn grain(declared: DeclaredGrain) -> Grain {
    match declared {
        DeclaredGrain::Day => Grain::Day,
        DeclaredGrain::Week => Grain::Week,
        DeclaredGrain::Month => Grain::Month,
        DeclaredGrain::Quarter => Grain::Quarter,
        DeclaredGrain::Year => Grain::Year,
    }
}

/// What the summary definition hash covers: the normalized summary extension, the base
/// `source`, and the tree and type of every base field it refers to (E0H-3.3).
fn definition(summary: &Summary, datasets: &[Dataset]) -> Json {
    let base = datasets.iter().find(|dataset| dataset.name == summary.of);
    let fields = read_fields(summary)
        .iter()
        .filter_map(|field| base.and_then(|base| base.field(field)))
        .map(field_definition)
        .collect();
    object([
        ("summary", extension(summary)),
        (
            "base_source",
            base.map_or(Json::Null, |base| text(&base.source)),
        ),
        ("fields", Json::Array(fields)),
    ])
}

/// The base fields a summary refers to: dimension `base` fields and the fields each
/// `base_aggregate` reads, ordered by name, each once.
fn read_fields(summary: &Summary) -> Vec<NormalizedName> {
    let mut read: Vec<NormalizedName> = summary
        .dimensions
        .iter()
        .map(|dimension| dimension.base.clone())
        .collect();
    for tree in summary
        .measures
        .iter()
        .filter_map(|measure| measure.base_aggregate.as_ref())
    {
        read.extend(
            ReadNames::of(&tree.json_form())
                .references
                .into_iter()
                .map(|(_, field)| NormalizedName::new(field.as_str())),
        );
    }
    read.sort();
    read.dedup();
    read
}

/// The normalized summary extension.
fn extension(summary: &Summary) -> Json {
    let dimensions = summary
        .dimensions
        .iter()
        .map(|dimension| {
            object([
                ("field", name(&dimension.field)),
                ("base", name(&dimension.base)),
                (
                    "grain",
                    dimension.grain.as_ref().map_or(Json::Null, serialized),
                ),
            ])
        })
        .collect();
    let measures = summary
        .measures
        .iter()
        .map(|measure| {
            object([
                ("field", name(&measure.field)),
                ("base_aggregate", tree(measure.base_aggregate.as_ref())),
            ])
        })
        .collect();
    object([
        ("dataset", name(&summary.dataset)),
        ("of", name(&summary.of)),
        ("dimensions", Json::Array(dimensions)),
        ("measures", Json::Array(measures)),
        ("coverage_field", name(&summary.coverage_field)),
    ])
}
