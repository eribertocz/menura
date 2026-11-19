//! The documents of metrics and their extensions.

use semantic_compiler_model::internal_model::{
    Additivity, ContextModifier, DeclaredAdditivity, DecomposabilityClass, DivisionByZero,
    FieldPath, Metric, MetricClassification, MissingHandling, NullHandling, RemovedDimensions,
    Selection, SemiAdditive, SnapshotAggregation, StaleHandling, UnexpectedHandling,
};
use serde_json::Value as Json;

use super::super::located::normalized_number;
use super::{codes, descriptions, json, name, names, object, serialized, text, tree, typed};

/// The whole meaning of a metric.
pub(in super::super) fn metric(metric: &Metric) -> Json {
    object([
        ("definition", metric_definition(metric)),
        ("type", typed(metric.typed.as_ref())),
        (
            "classification",
            metric
                .classification
                .as_ref()
                .map_or(Json::Null, classification),
        ),
        (
            "derived",
            metric.derived.as_ref().map_or(Json::Null, |derived| {
                object([
                    ("inputs", names(&derived.inputs)),
                    (
                        "on_division_by_zero",
                        derived.on_division_by_zero.map_or(Json::Null, |value| {
                            text(match value {
                                DivisionByZero::Null => "null",
                                DivisionByZero::Error => "error",
                            })
                        }),
                    ),
                ])
            }),
        ),
        (
            "context",
            metric.context.as_ref().map_or(Json::Null, |context| {
                object([
                    ("base", name(&context.base)),
                    ("modifier", modifier(&context.modifier)),
                ])
            }),
        ),
        ("descriptions", descriptions(&metric.descriptions)),
        ("query_errors", codes(&metric.query_errors)),
    ])
}

/// What a `semi_additive` check reads of a metric: its name, normalized tree and additivity
/// extension (CHK-3).
pub(in super::super) fn metric_definition(metric: &Metric) -> Json {
    object([
        ("name", name(&metric.name)),
        ("expression", tree(metric.expression.as_ref())),
        (
            "additivity",
            metric
                .declared_additivity
                .as_ref()
                .map_or(Json::Null, additivity),
        ),
    ])
}

fn classification(classification: &MetricClassification) -> Json {
    let components = classification
        .components
        .iter()
        .map(|component| {
            object([
                ("call", json(&component.call.json_form())),
                ("filter", tree(component.filter.as_ref())),
                ("home_dataset", name(&component.home_dataset)),
                (
                    "class",
                    text(match component.class {
                        DecomposabilityClass::Distributive => "distributive",
                        DecomposabilityClass::Algebraic => "algebraic",
                        DecomposabilityClass::Holistic => "holistic",
                    }),
                ),
            ])
        })
        .collect();
    object([
        ("components", Json::Array(components)),
        ("inputs", names(&classification.inputs)),
        ("home_datasets", names(&classification.home_datasets)),
        (
            "additivity",
            text(match classification.additivity {
                Additivity::Additive => "additive",
                Additivity::NonAdditive => "non_additive",
                Additivity::SemiAdditive => "semi_additive",
            }),
        ),
        ("recomposable", Json::Bool(classification.recomposable)),
    ])
}

fn field_path(path: &FieldPath) -> Json {
    object([
        ("dataset", name(&path.dataset)),
        ("field", name(&path.field)),
    ])
}

fn additivity(declared: &DeclaredAdditivity) -> Json {
    match declared {
        DeclaredAdditivity::NonAdditive => object([("type", text("non_additive"))]),
        DeclaredAdditivity::SemiAdditive(keys) => semi_additive(keys),
    }
}

fn semi_additive(keys: &SemiAdditive) -> Json {
    object([
        ("type", text("semi_additive")),
        ("over", field_path(&keys.over)),
        (
            "entity",
            Json::Array(keys.entity.iter().map(field_path).collect()),
        ),
        ("aggregation", aggregation(keys.aggregation)),
        (
            "nulls",
            text(match keys.nulls {
                NullHandling::AsMissing => "as_missing",
                NullHandling::Keep => "keep",
                NullHandling::Zero => "zero",
                NullHandling::Error => "error",
            }),
        ),
        (
            "snapshot_grain",
            keys.snapshot_grain.as_ref().map_or(Json::Null, serialized),
        ),
        (
            "population",
            keys.population.as_deref().map_or(Json::Null, text),
        ),
        ("missing", keys.missing.map_or(Json::Null, missing)),
        (
            "unexpected",
            keys.unexpected.map_or(Json::Null, |unexpected| {
                text(match unexpected {
                    UnexpectedHandling::Include => "include",
                    UnexpectedHandling::Exclude => "exclude",
                    UnexpectedHandling::Error => "error",
                })
            }),
        ),
    ])
}

fn aggregation(aggregation: SnapshotAggregation) -> Json {
    let selection = |selection: Selection| {
        text(match selection {
            Selection::GlobalPeriod => "global_period",
            Selection::PerEntity => "per_entity",
        })
    };
    match aggregation {
        SnapshotAggregation::Last(selected) => object([
            ("function", text("last")),
            ("selection", selection(selected)),
        ]),
        SnapshotAggregation::First(selected) => object([
            ("function", text("first")),
            ("selection", selection(selected)),
        ]),
        SnapshotAggregation::Minimum => object([("function", text("min"))]),
        SnapshotAggregation::Maximum => object([("function", text("max"))]),
        SnapshotAggregation::Average => object([("function", text("avg"))]),
    }
}

fn missing(missing: MissingHandling) -> Json {
    match missing {
        MissingHandling::Exclude => object([("handling", text("exclude"))]),
        MissingHandling::CarryForward {
            max_carry_forward,
            on_stale,
        } => object([
            ("handling", text("carry_forward")),
            (
                "max_carry_forward",
                normalized_number(max_carry_forward.get()),
            ),
            (
                "on_stale",
                text(match on_stale {
                    StaleHandling::Exclude => "exclude",
                    StaleHandling::Error => "error",
                }),
            ),
        ]),
        MissingHandling::Zero => object([("handling", text("zero"))]),
        MissingHandling::Error => object([("handling", text("error"))]),
    }
}

fn modifier(modifier: &ContextModifier) -> Json {
    match modifier {
        ContextModifier::PeriodToDate {
            time_dimension,
            grain,
        } => object([
            ("type", text("period_to_date")),
            ("time_dimension", field_path(time_dimension)),
            ("grain", serialized(grain)),
        ]),
        ContextModifier::PriorPeriod {
            time_dimension,
            grain,
            offset,
        } => object([
            ("type", text("prior_period")),
            ("time_dimension", field_path(time_dimension)),
            ("grain", serialized(grain)),
            ("offset", normalized_number(offset.get())),
        ]),
        ContextModifier::ShareOfTotal(RemovedDimensions::All) => {
            object([("type", text("share_of_total")), ("remove", text("all"))])
        }
        ContextModifier::ShareOfTotal(RemovedDimensions::Fields(fields)) => object([
            ("type", text("share_of_total")),
            (
                "remove",
                Json::Array(fields.iter().map(field_path).collect()),
            ),
        ]),
    }
}
