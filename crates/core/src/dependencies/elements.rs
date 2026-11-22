//! One entry per dataset, relationship, field, metric and row policy (DEP-3.2), each list
//! ordered by `object`.

use std::collections::BTreeSet;

use menura_contracts::Sha256Hash;
use menura_contracts::types::dependencies::{ElementDependencies, MetricDependencies};
use menura_model::internal_model::{DiagnosticCode, FieldPath, NormalizedName, Relationship};

use super::hashes::{dataset_hash, field_hash, metric_hash, policy_hash, relationship_hash};
use super::inputs::Inputs;
use super::metrics::{find, metric_inputs, upstream_codes};
use super::names::{MetricName, ModelInput, PolicyName};
use super::resolve::Resolver;
use super::usability::{UNREACHABLE_ROW_ACCESS, own_codes, query_error_code};
use crate::describe::metrics::kind;
use crate::lineage::policies::normalized_policy_name;

/// One entry for every dataset, summaries included (DEP-3.5).
pub(super) fn datasets(resolver: &Resolver<'_>) -> Vec<ElementDependencies> {
    let propagated = BTreeSet::from([DiagnosticCode::new(UNREACHABLE_ROW_ACCESS)]);
    let mut entries: Vec<(ModelInput, ElementDependencies)> = resolver
        .model()
        .parts()
        .datasets
        .iter()
        .map(|dataset| {
            let itself = ModelInput::Dataset(dataset.name.clone());
            let own = own_codes(&dataset.query_errors, &propagated);
            let hash = dataset_hash(dataset, resolver.summary(&dataset.name), &own);
            let inputs = resolver.dataset(&dataset.name);
            let entry = entry(&itself, hash, &dataset.query_errors, &inputs);
            (itself, entry)
        })
        .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries.into_iter().map(|(_, entry)| entry).collect()
}

/// One entry for every relationship, usable or not (DEP-3.8).
pub(super) fn relationships(resolver: &Resolver<'_>) -> Vec<ElementDependencies> {
    let mut entries: Vec<(ModelInput, ElementDependencies)> = resolver
        .model()
        .parts()
        .relationships
        .iter()
        .map(|relationship| {
            let itself = ModelInput::Relationship(relationship.name.clone());
            let (inputs, upstream) = relationship_inputs(resolver, relationship);
            let own = own_codes(&relationship.query_errors, &upstream);
            let hash = relationship_hash(relationship, &own);
            let mut entry = entry(&itself, hash, &relationship.query_errors, &inputs);
            entry.security_policies = Vec::new();
            (itself, entry)
        })
        .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries.into_iter().map(|(_, entry)| entry).collect()
}

/// The join-key fields of `relationship` with their inputs, or the source columns a key names
/// (E0D-3.1); and the query error codes of those fields.
fn relationship_inputs(
    resolver: &Resolver<'_>,
    relationship: &Relationship,
) -> (Inputs, BTreeSet<DiagnosticCode>) {
    let mut inputs = Inputs::default();
    let mut upstream = BTreeSet::new();
    let keys = relationship
        .from_columns
        .iter()
        .map(|key| (&relationship.from, key))
        .chain(
            relationship
                .to_columns
                .iter()
                .map(|key| (&relationship.to, key)),
        );
    for (dataset, key) in keys {
        let field = resolver
            .model()
            .dataset(dataset)
            .and_then(|found| found.field(key));
        match field {
            Some(field) => {
                upstream.extend(field.query_errors.iter().cloned());
                inputs.absorb(&resolver.field(&FieldPath {
                    dataset: dataset.clone(),
                    field: key.clone(),
                }));
            }
            None => inputs.absorb(&resolver.key(dataset, key)),
        }
    }
    (inputs, upstream)
}

/// One entry for every field of every dataset (DEP-3.6).
pub(super) fn fields(resolver: &Resolver<'_>) -> Vec<ElementDependencies> {
    let mut entries = Vec::new();
    for dataset in &resolver.model().parts().datasets {
        for field in &dataset.fields {
            let path = FieldPath {
                dataset: dataset.name.clone(),
                field: field.name.clone(),
            };
            let itself = ModelInput::Field(path.clone());
            let own: Vec<&DiagnosticCode> = field.query_errors.iter().collect();
            let hash = field_hash(field, &own);
            let inputs = resolver.field(&path);
            entries.push((
                itself.clone(),
                entry(&itself, hash, &field.query_errors, &inputs),
            ));
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries.into_iter().map(|(_, entry)| entry).collect()
}

/// One entry for every model metric and every dataset metric (DEP-3.7).
pub(super) fn metrics(resolver: &Resolver<'_>) -> Vec<MetricDependencies> {
    let model = resolver.model();
    let mut names: Vec<MetricName> = model
        .parts()
        .metrics
        .iter()
        .map(|metric| MetricName::of_model(&metric.name))
        .chain(model.parts().datasets.iter().flat_map(|dataset| {
            dataset
                .metrics
                .iter()
                .map(|metric| MetricName::of_dataset(&dataset.name, &metric.name))
        }))
        .collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let metric = find(resolver, &name)?;
            let upstream = upstream_codes(resolver, &name, metric);
            let own = own_codes(&metric.query_errors, &upstream);
            let hash = metric_hash(metric, &own);
            let inputs = metric_inputs(resolver, &name);
            let itself = ModelInput::Metric(name);
            let common = entry(&itself, hash, &metric.query_errors, &inputs);
            Some(MetricDependencies {
                object: common.object,
                kind: kind(metric),
                definition_hash: common.definition_hash,
                usable: common.usable,
                query_error_code: common.query_error_code,
                model_inputs: common.model_inputs,
                physical_inputs: common.physical_inputs,
                security_policies: common.security_policies,
            })
        })
        .collect()
}

/// One entry for every row policy, with the usability of its dataset (DEP-3.3, DEP-3.8).
pub(super) fn policies(resolver: &Resolver<'_>) -> Vec<ElementDependencies> {
    let mut entries = Vec::new();
    for dataset in &resolver.model().parts().datasets {
        for policy in &dataset.row_policies {
            let name = PolicyName {
                dataset: dataset.name.clone(),
                policy: NormalizedName::new(&normalized_policy_name(&policy.name)),
            };
            let mut inputs = Inputs::default();
            inputs
                .model
                .insert(ModelInput::Dataset(dataset.name.clone()));
            if let Some(condition) = &policy.condition {
                inputs.absorb(&resolver.row_fields(&dataset.name, condition));
            }
            let entry = ElementDependencies {
                object: format!("policy:{}", name.written()),
                definition_hash: policy_hash(policy),
                usable: dataset.query_errors.is_empty(),
                query_error_code: query_error_code(&dataset.query_errors),
                model_inputs: inputs.model.iter().map(ModelInput::written).collect(),
                physical_inputs: inputs.written_physical_inputs(),
                security_policies: Vec::new(),
            };
            entries.push((name, entry));
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries.into_iter().map(|(_, entry)| entry).collect()
}

/// The entry of the element `itself`, with the query error codes `query_errors` and `inputs`.
fn entry(
    itself: &ModelInput,
    definition_hash: Sha256Hash,
    query_errors: &[DiagnosticCode],
    inputs: &Inputs,
) -> ElementDependencies {
    ElementDependencies {
        object: itself.written(),
        definition_hash,
        usable: query_errors.is_empty(),
        query_error_code: query_error_code(query_errors),
        model_inputs: inputs.written_model_inputs(itself),
        physical_inputs: inputs.written_physical_inputs(),
        security_policies: inputs.written_policies(),
    }
}
