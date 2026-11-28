//! Definition hashes (DEP-3.10): the CON-2 hash of an element's own normalized tree, without
//! labels, descriptions, `ai_context`, another element's definition or the element's position.

use std::num::NonZeroU64;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_contracts::{Sha256Hash, hash_value};
use menura_model::internal_model::{
    AccessMode, ContextModifier, Dataset, DatasetAccess, DeclaredAdditivity, DiagnosticCode,
    DivisionByZero, Field, FieldPath, Metric, MissingHandling, NormalizedName, NullHandling,
    Population, PopulationMode, Relationship, RemovedDimensions, RoleName, RowPolicy, Selection,
    SemiAdditive, SnapshotAggregation, StaleHandling, Summary, TypedValue, UnexpectedHandling,
    ValidToBound, ValueType,
};
use serde_json::{Value, json};

use crate::json_form::json_value;
use crate::lineage::policies::normalized_policy_name;
use menura_model::expression::Expression;

/// The definition hash of `dataset`: its `source`, keys, `access`, populations and summary
/// mapping, never its fields, metrics or policies; with its own query error codes `own`.
pub(super) fn dataset_hash(
    dataset: &Dataset,
    summary: Option<&Summary>,
    own: &[&DiagnosticCode],
) -> Sha256Hash {
    hash_value(&json!({
        "name": dataset.name.as_str(),
        "source": dataset.source,
        "primary_key": names(&dataset.primary_key),
        "unique_keys": dataset.unique_keys.iter().map(|key| names(key)).collect::<Vec<_>>(),
        "access": dataset.access.as_ref().map_or(Value::Null, access),
        "populations": dataset.populations.iter().map(population).collect::<Vec<_>>(),
        "summary": summary.map_or(Value::Null, summary_mapping),
        "query_errors": codes(own),
    }))
}

/// The definition hash of `field`: its expression and logical type, with its own query error
/// codes `own`.
pub(super) fn field_hash(field: &Field, own: &[&DiagnosticCode]) -> Sha256Hash {
    hash_value(&json!({
        "name": field.name.as_str(),
        "expression": tree(field.expression.as_ref()),
        "type": typed(field.typed.as_ref()),
        "query_errors": codes(own),
    }))
}

/// The definition hash of `metric`: its expression and extensions, with its own query error
/// codes `own`.
pub(super) fn metric_hash(metric: &Metric, own: &[&DiagnosticCode]) -> Sha256Hash {
    let derived = metric.derived.as_ref().map_or(Value::Null, |derived| {
        json!({
            "inputs": names(&derived.inputs),
            "on_division_by_zero": derived.on_division_by_zero.map(|value| match value {
                DivisionByZero::Null => "null",
                DivisionByZero::Error => "error",
            }),
        })
    });
    let context = metric.context.as_ref().map_or(
        Value::Null,
        |context| json!({"base": context.base.as_str(), "modifier": modifier(&context.modifier)}),
    );
    hash_value(&json!({
        "name": metric.name.as_str(),
        "expression": tree(metric.expression.as_ref()),
        "additivity": metric.declared_additivity.as_ref().map_or(Value::Null, additivity),
        "derived": derived,
        "context": context,
        "query_errors": codes(own),
    }))
}

/// The definition hash of `relationship`: its datasets and keys, with its own query error
/// codes `own`.
pub(super) fn relationship_hash(
    relationship: &Relationship,
    own: &[&DiagnosticCode],
) -> Sha256Hash {
    hash_value(&json!({
        "name": relationship.name.as_str(),
        "from": relationship.from.as_str(),
        "to": relationship.to.as_str(),
        "from_columns": names(&relationship.from_columns),
        "to_columns": names(&relationship.to_columns),
        "query_errors": codes(own),
    }))
}

/// The definition hash of `policy`: its normalized name (E0D-1.4), condition, roles and
/// exemptions.
pub(super) fn policy_hash(policy: &RowPolicy) -> Sha256Hash {
    hash_value(&json!({
        "name": normalized_policy_name(&policy.name),
        "condition": tree(policy.condition.as_ref()),
        "roles": policy.roles.as_deref().map(roles),
        "exempt_roles": policy.exempt_roles.as_deref().map(roles),
    }))
}

fn tree(expression: Option<&Expression>) -> Value {
    expression.map_or(Value::Null, |expression| {
        json_value(&expression.json_form())
    })
}

fn names(names: &[NormalizedName]) -> Vec<&str> {
    names.iter().map(NormalizedName::as_str).collect()
}

fn codes<'code>(codes: &[&'code DiagnosticCode]) -> Vec<&'code str> {
    codes.iter().map(|code| code.as_str()).collect()
}

fn roles(roles: &[RoleName]) -> Vec<&str> {
    roles.iter().map(RoleName::as_str).collect()
}

fn grain_value(grain: Grain) -> Value {
    serde_json::to_value(grain).unwrap_or(Value::Null)
}

fn logical(logical: &LogicalType) -> Value {
    serde_json::to_value(logical).unwrap_or(Value::Null)
}

fn typed(typed: Option<&TypedValue>) -> Value {
    typed.map_or(Value::Null, |typed| {
        let value_type = match &typed.value_type {
            ValueType::Null => Value::String("null".to_owned()),
            ValueType::Logical(found) => logical(found),
        };
        json!({"type": value_type, "nullable": typed.nullable})
    })
}

fn access(access: &DatasetAccess) -> Value {
    let mode = match &access.mode {
        AccessMode::Authenticated => json!({"mode": "authenticated"}),
        AccessMode::Roles(list) => json!({"mode": "roles", "roles": roles(list)}),
        AccessMode::Internal => json!({"mode": "internal"}),
    };
    let fields: Vec<Value> = access
        .fields
        .iter()
        .map(|restriction| {
            json!({"fields": names(&restriction.fields), "roles": roles(&restriction.roles)})
        })
        .collect();
    json!({"mode": mode, "fields": fields})
}

fn population(population: &Population) -> Value {
    let mode = match &population.mode {
        PopulationMode::Validity {
            valid_from,
            valid_to,
            partial_period,
            valid_to_bound,
            open_end,
        } => {
            let mut validity = json!({
                "mode": "validity",
                "valid_from": valid_from.as_str(),
                "valid_to": valid_to.as_str(),
                "partial_period": partial_period.as_str(),
            });
            // The new keys join the tree only when they differ from their defaults, so the
            // hash of a population without them is unchanged (VAL-1.1, VAL-1.2).
            if *valid_to_bound != ValidToBound::Inclusive {
                validity["valid_to_bound"] = json!(valid_to_bound.as_str());
            }
            if let Some(open_end) = open_end {
                validity["open_end"] = json!(open_end.con4_text());
            }
            validity
        }
        PopulationMode::Periodic { period, grain } => {
            json!({"mode": "periodic", "period": period.as_str(), "grain": grain_value(*grain)})
        }
    };
    json!({
        "name": population.name.as_str(),
        "entity": names(&population.entity),
        "mode": mode,
        "required_when": tree(population.required_when.as_ref()),
    })
}

/// The `aggregate` extension of a summary dataset, without the summary definition hash, which
/// covers the definitions of base fields (E0H-3.3).
fn summary_mapping(summary: &Summary) -> Value {
    let dimensions: Vec<Value> = summary
        .dimensions
        .iter()
        .map(|dimension| {
            json!({
                "field": dimension.field.as_str(),
                "base": dimension.base.as_str(),
                "grain": dimension.grain.as_ref().copied().map(grain_value),
            })
        })
        .collect();
    let measures: Vec<Value> = summary
        .measures
        .iter()
        .map(|measure| {
            json!({
                "field": measure.field.as_str(),
                "base_aggregate": tree(measure.base_aggregate.as_ref()),
            })
        })
        .collect();
    json!({
        "of": summary.of.as_str(),
        "dimensions": dimensions,
        "measures": measures,
        "coverage_field": summary.coverage_field.as_str(),
    })
}

fn field_path(path: &FieldPath) -> Value {
    json!({"dataset": path.dataset.as_str(), "field": path.field.as_str()})
}

fn additivity(declared: &DeclaredAdditivity) -> Value {
    match declared {
        DeclaredAdditivity::NonAdditive => json!({"type": "non_additive"}),
        DeclaredAdditivity::SemiAdditive(keys) => semi_additive(keys),
    }
}

fn semi_additive(keys: &SemiAdditive) -> Value {
    json!({
        "type": "semi_additive",
        "over": field_path(&keys.over),
        "entity": keys.entity.iter().map(field_path).collect::<Vec<_>>(),
        "aggregation": aggregation(keys.aggregation),
        "nulls": match keys.nulls {
            NullHandling::AsMissing => "as_missing",
            NullHandling::Keep => "keep",
            NullHandling::Zero => "zero",
            NullHandling::Error => "error",
        },
        "snapshot_grain": keys.snapshot_grain.as_ref().copied().map(grain_value),
        "population": keys.population.as_ref().map(NormalizedName::as_str),
        "missing": keys.missing.map(missing),
        "unexpected": keys.unexpected.map(|unexpected| match unexpected {
            UnexpectedHandling::Include => "include",
            UnexpectedHandling::Exclude => "exclude",
            UnexpectedHandling::Error => "error",
        }),
    })
}

fn aggregation(aggregation: SnapshotAggregation) -> Value {
    let selection = |selection: Selection| match selection {
        Selection::GlobalPeriod => "global_period",
        Selection::PerEntity => "per_entity",
    };
    match aggregation {
        SnapshotAggregation::Last(selected) => {
            json!({"function": "last", "selection": selection(selected)})
        }
        SnapshotAggregation::First(selected) => {
            json!({"function": "first", "selection": selection(selected)})
        }
        SnapshotAggregation::Minimum => json!({"function": "min"}),
        SnapshotAggregation::Maximum => json!({"function": "max"}),
        SnapshotAggregation::Average => json!({"function": "avg"}),
    }
}

/// A positive count as an exact string (CON-4), whatever its size.
fn count(value: NonZeroU64) -> String {
    value.get().to_string()
}

fn missing(missing: MissingHandling) -> Value {
    match missing {
        MissingHandling::Exclude => json!({"handling": "exclude"}),
        MissingHandling::CarryForward {
            max_carry_forward,
            on_stale,
        } => json!({
            "handling": "carry_forward",
            "max_carry_forward": count(max_carry_forward),
            "on_stale": match on_stale {
                StaleHandling::Exclude => "exclude",
                StaleHandling::Error => "error",
            },
        }),
        MissingHandling::Zero => json!({"handling": "zero"}),
        MissingHandling::Error => json!({"handling": "error"}),
    }
}

fn modifier(modifier: &ContextModifier) -> Value {
    match modifier {
        ContextModifier::PeriodToDate {
            time_dimension,
            grain,
        } => json!({
            "type": "period_to_date",
            "time_dimension": field_path(time_dimension),
            "grain": grain_value(*grain),
        }),
        ContextModifier::PriorPeriod {
            time_dimension,
            grain,
            offset,
        } => json!({
            "type": "prior_period",
            "time_dimension": field_path(time_dimension),
            "grain": grain_value(*grain),
            "offset": count(*offset),
        }),
        ContextModifier::ShareOfTotal(RemovedDimensions::All) => {
            json!({"type": "share_of_total", "remove": "all"})
        }
        ContextModifier::ShareOfTotal(RemovedDimensions::Fields(fields)) => json!({
            "type": "share_of_total",
            "remove": fields.iter().map(field_path).collect::<Vec<_>>(),
        }),
    }
}
