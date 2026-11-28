//! The inputs of fields, join keys and datasets (DEP-3.4 to DEP-3.6), computed once per model.

use std::collections::{BTreeMap, BTreeSet};

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::{CatalogSchemaDocument, RelationSchema};
use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, FieldPath, InternalModel, NormalizedName, Summary};

use super::inputs::{CatalogColumn, Inputs};
use super::names::{ModelInput, PolicyName};
use super::visibility::PolicyGraph;
use crate::lineage::policies::normalized_policy_name;
use crate::lineage::references::{normalized_name, plain_references};

/// The model and catalog schema the inputs are read from, with the inputs of every dataset.
pub(super) struct Resolver<'model> {
    model: &'model InternalModel,
    catalog: &'model CatalogSchemaDocument,
    datasets: BTreeMap<NormalizedName, Inputs>,
    visibility: BTreeMap<NormalizedName, Inputs>,
}

impl<'model> Resolver<'model> {
    /// The resolver of `model`, loaded against `catalog`.
    pub(super) fn new(
        model: &'model InternalModel,
        catalog: &'model CatalogSchemaDocument,
    ) -> Self {
        let mut resolver = Self {
            model,
            catalog,
            datasets: BTreeMap::new(),
            visibility: BTreeMap::new(),
        };
        let graph = PolicyGraph::new(model);
        let visibility: BTreeMap<NormalizedName, Inputs> = model
            .parts()
            .datasets
            .iter()
            .map(|dataset| (dataset.name.clone(), resolver.visibility(&graph, dataset)))
            .collect();
        resolver.datasets = model
            .parts()
            .datasets
            .iter()
            .map(|dataset| {
                let inputs = resolver.dataset_inputs(dataset, &visibility);
                (dataset.name.clone(), inputs)
            })
            .collect();
        resolver.visibility = visibility;
        resolver
    }

    /// The model.
    pub(super) fn model(&self) -> &'model InternalModel {
        self.model
    }

    /// The inputs of the dataset `name` (DEP-3.5); empty for a dataset the model does not have.
    pub(super) fn dataset(&self, name: &NormalizedName) -> Inputs {
        self.datasets.get(name).cloned().unwrap_or_default()
    }

    /// The `sources` of the dataset `name` (DEP-3.5): its own `source`; for a summary, those of
    /// the datasets of the visibility inputs of its base; for any other dataset, those of the
    /// summaries of it and of the datasets of its visibility inputs. Without duplicates, in
    /// code-point order.
    pub(super) fn sources(&self, name: &NormalizedName) -> Vec<String> {
        let mut sources = BTreeSet::new();
        let Some(dataset) = self.model.dataset(name) else {
            return Vec::new();
        };
        sources.insert(dataset.source.clone());
        let visible_from = if let Some(summary) = self.summary(name) {
            &summary.of
        } else {
            sources.extend(self.summary_sources(name));
            name
        };
        let datasets = self
            .visibility
            .get(visible_from)
            .into_iter()
            .flat_map(|inputs| inputs.model.iter())
            .filter_map(|input| match input {
                ModelInput::Dataset(reached) => self.model.dataset(reached),
                _ => None,
            });
        sources.extend(datasets.map(|reached| reached.source.clone()));
        sources.into_iter().collect()
    }

    /// The `source` of every summary dataset whose `aggregate.of` names the dataset `base`.
    fn summary_sources(&self, base: &NormalizedName) -> Vec<String> {
        self.model
            .parts()
            .summaries
            .iter()
            .filter(|summary| &summary.of == base)
            .filter_map(|summary| self.model.dataset(&summary.dataset))
            .map(|summary| summary.source.clone())
            .collect()
    }

    /// The inputs of the field `path` (DEP-3.6), the field itself included: its dataset, its
    /// columns and the inputs of its dataset.
    pub(super) fn field(&self, path: &FieldPath) -> Inputs {
        let mut inputs = self.field_alone(path);
        inputs
            .model
            .insert(ModelInput::Dataset(path.dataset.clone()));
        inputs.absorb(&self.dataset(&path.dataset));
        inputs
    }

    /// The field `path` and the catalog columns its expression reads, without the inputs of
    /// its dataset; empty for a field the model does not have.
    pub(super) fn field_alone(&self, path: &FieldPath) -> Inputs {
        let mut inputs = Inputs::default();
        let Some(dataset) = self.model.dataset(&path.dataset) else {
            return inputs;
        };
        let Some(field) = dataset.field(&path.field) else {
            return inputs;
        };
        inputs.model.insert(ModelInput::Field(path.clone()));
        for reference in field.expression.iter().flat_map(plain_references) {
            if let Some(column) = self.column(dataset, reference.name.text()) {
                inputs.physical.insert(column);
            }
        }
        inputs
    }

    /// What the join key `name` of `dataset` reads: the field of that name alone, or else the
    /// source column of that name (E0D-3.1).
    pub(super) fn key(&self, dataset: &NormalizedName, name: &NormalizedName) -> Inputs {
        let path = FieldPath {
            dataset: dataset.clone(),
            field: name.clone(),
        };
        let Some(found) = self.model.dataset(dataset) else {
            return Inputs::default();
        };
        if found.field(name).is_some() {
            return self.field_alone(&path);
        }
        let mut inputs = Inputs::default();
        inputs.physical.extend(self.column(found, name.as_str()));
        inputs
    }

    /// The bare field references of `expression`, a row-level expression of `dataset`, each
    /// with the columns it reads.
    pub(super) fn row_fields(&self, dataset: &NormalizedName, expression: &Expression) -> Inputs {
        let mut inputs = Inputs::default();
        for path in row_paths(dataset, expression) {
            inputs.absorb(&self.field_alone(&path));
        }
        inputs
    }

    /// The column `name` of the catalog relation of `dataset`, compared as E0D-1.5 states, in
    /// the catalog's spelling (PLN-4.5).
    pub(super) fn column(&self, dataset: &Dataset, name: &str) -> Option<CatalogColumn> {
        let relation = self.relation(dataset)?;
        let wanted = normalize_unquoted_identifier(name);
        relation
            .columns
            .iter()
            .find(|column| normalize_unquoted_identifier(&column.name) == wanted)
            .map(|column| CatalogColumn {
                source: relation.source.clone(),
                column: column.name.clone(),
            })
    }

    fn relation(&self, dataset: &Dataset) -> Option<&'model RelationSchema> {
        self.catalog
            .relations
            .iter()
            .find(|relation| relation.source == dataset.source)
    }

    /// The visibility inputs of `dataset` (DEP-3.4): its own policies, and along every
    /// relationship V(D) follows, the relationship, its join keys, and the policies of the
    /// datasets reached.
    fn visibility(&self, graph: &PolicyGraph<'model>, dataset: &'model Dataset) -> Inputs {
        let mut inputs = Inputs::default();
        let mut chain = vec![&dataset.name];
        self.visit(graph, dataset, &mut chain, &mut inputs);
        inputs
    }

    fn visit(
        &self,
        graph: &PolicyGraph<'model>,
        dataset: &'model Dataset,
        chain: &mut Vec<&'model NormalizedName>,
        inputs: &mut Inputs,
    ) {
        inputs.absorb(&self.own_policies(dataset));
        for edge in graph.followed(&dataset.name, chain) {
            inputs
                .model
                .insert(ModelInput::Relationship(edge.relationship.clone()));
            if let Some(relationship) = self.model.relationship(&edge.relationship) {
                for key in &relationship.from_columns {
                    inputs.absorb(&self.key(&relationship.from, key));
                }
                for key in &relationship.to_columns {
                    inputs.absorb(&self.key(&relationship.to, key));
                }
            }
            let Some(target) = self.model.dataset(&edge.to) else {
                continue;
            };
            if !target.row_policies.is_empty() {
                inputs
                    .model
                    .insert(ModelInput::Dataset(target.name.clone()));
            }
            chain.push(&target.name);
            self.visit(graph, target, chain, inputs);
            chain.pop();
        }
    }

    /// The row policies of `dataset`, with the fields their conditions read.
    fn own_policies(&self, dataset: &Dataset) -> Inputs {
        let mut inputs = Inputs::default();
        for policy in &dataset.row_policies {
            inputs.policies.insert(PolicyName {
                dataset: dataset.name.clone(),
                policy: NormalizedName::new(&normalized_policy_name(&policy.name)),
            });
            if let Some(condition) = &policy.condition {
                inputs.absorb(&self.row_fields(&dataset.name, condition));
            }
        }
        inputs
    }

    /// The inputs of the dataset entry (DEP-3.5): its visibility inputs and, for a summary,
    /// its base, the visibility inputs of the base and the base fields its mapping names, with
    /// the policies of V(base) only.
    fn dataset_inputs(
        &self,
        dataset: &Dataset,
        visibility: &BTreeMap<NormalizedName, Inputs>,
    ) -> Inputs {
        let mut inputs = visibility.get(&dataset.name).cloned().unwrap_or_default();
        let Some(summary) = self.summary(&dataset.name) else {
            return inputs;
        };
        let base = visibility.get(&summary.of).cloned().unwrap_or_default();
        inputs.policies.clear();
        inputs.absorb(&base);
        inputs.model.insert(ModelInput::Dataset(summary.of.clone()));
        for dimension in &summary.dimensions {
            inputs.absorb(&self.field_alone(&FieldPath {
                dataset: summary.of.clone(),
                field: dimension.base.clone(),
            }));
        }
        for aggregate in summary
            .measures
            .iter()
            .filter_map(|measure| measure.base_aggregate.as_ref())
        {
            inputs.absorb(&self.row_fields(&summary.of, aggregate));
        }
        inputs
    }

    /// The summary whose summary dataset is `dataset`, if any.
    pub(super) fn summary(&self, dataset: &NormalizedName) -> Option<&'model Summary> {
        self.model
            .parts()
            .summaries
            .iter()
            .find(|summary| &summary.dataset == dataset)
    }
}

/// The fields of `dataset` that the bare references of `expression`, a row-level expression of
/// that dataset, name (SCY-5.1.6, SUM-2).
pub(super) fn row_paths(dataset: &NormalizedName, expression: &Expression) -> Vec<FieldPath> {
    plain_references(expression)
        .into_iter()
        .map(|reference| FieldPath {
            dataset: dataset.clone(),
            field: normalized_name(&reference.name),
        })
        .collect()
}
