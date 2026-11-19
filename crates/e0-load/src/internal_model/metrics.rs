//! The metrics of the internal model (E0H-2 "Metrics").

use semantic_compiler_model::internal_model::{
    Additivity, Component, ContextMetric, DecomposabilityClass, DerivedMetric, Metric,
    MetricClassification,
};

use super::assembly::Assembly;
use super::declarations::{context_modifier, declared_additivity, division_by_zero};
use super::located::{declared, descriptions, location_of, normalized};
use super::trees::{normalized as normalized_tree, normalized_call};
use crate::classification::{self, ClassifiedMetric};
use crate::expressions::ExpressionPlace;
use crate::outline::ElementOutline;
use crate::pointer::JsonPointer;

/// The metric `element`, of the semantic model or dataset-scoped.
pub(super) fn metric(assembly: &Assembly<'_, '_>, element: &ElementOutline<'_>) -> Metric {
    let pointer = &element.pointer;
    let input = assembly.input;
    Metric {
        name: declared(element.name.as_ref()),
        expression: assembly.tree(pointer, ExpressionPlace::Metric),
        typed: assembly.typed(pointer),
        classification: input
            .classification
            .metrics
            .iter()
            .find(|metric| metric.pointer == *pointer)
            .map(|metric| classified(assembly, metric)),
        declared_additivity: input
            .extensions
            .additivity
            .iter()
            .find(|declaration| declaration.metric == *pointer)
            .map(|declaration| declared_additivity(&declaration.additivity)),
        derived: derived(assembly, pointer),
        context: input.context.metric(pointer).map(|context| ContextMetric {
            base: metric_name(assembly, &context.base),
            modifier: context_modifier(&context.modifier),
        }),
        descriptions: descriptions(assembly.document, pointer),
        query_errors: assembly.usability.codes(pointer),
        location: location_of(assembly.document, pointer),
    }
}

/// The derived-metric extension of the metric at `pointer`, with the inputs E0.d resolved.
fn derived(assembly: &Assembly<'_, '_>, pointer: &JsonPointer) -> Option<DerivedMetric> {
    let input = assembly.input;
    let declaration = input.extensions.derived_declaration(pointer)?;
    let inputs = input
        .derived
        .metric(pointer)
        .map(|metric| {
            metric
                .inputs
                .iter()
                .map(|read| metric_name(assembly, &read.metric))
                .collect()
        })
        .unwrap_or_default();
    Some(DerivedMetric {
        inputs,
        on_division_by_zero: declaration.on_division_by_zero.map(division_by_zero),
    })
}

fn metric_name(
    assembly: &Assembly<'_, '_>,
    pointer: &JsonPointer,
) -> semantic_compiler_model::internal_model::NormalizedName {
    assembly
        .index
        .name_at(pointer)
        .map_or_else(|| declared(None), normalized)
}

fn classified(assembly: &Assembly<'_, '_>, metric: &ClassifiedMetric) -> MetricClassification {
    MetricClassification {
        components: metric
            .components
            .iter()
            .map(|component| Component {
                call: normalized_call(&component.call),
                filter: component.filter.as_ref().map(normalized_tree),
                home_dataset: normalized(&component.home_dataset),
                class: match component.class {
                    classification::DecomposabilityClass::Distributive => {
                        DecomposabilityClass::Distributive
                    }
                    classification::DecomposabilityClass::Algebraic => {
                        DecomposabilityClass::Algebraic
                    }
                    classification::DecomposabilityClass::Holistic => {
                        DecomposabilityClass::Holistic
                    }
                },
            })
            .collect(),
        inputs: metric
            .inputs
            .iter()
            .map(|input| metric_name(assembly, input))
            .collect(),
        home_datasets: metric.home_datasets.iter().map(normalized).collect(),
        additivity: match metric.additivity {
            classification::Additivity::Additive => Additivity::Additive,
            classification::Additivity::NonAdditive => Additivity::NonAdditive,
            classification::Additivity::SemiAdditive => Additivity::SemiAdditive,
        },
        recomposable: metric.recomposable,
    }
}
