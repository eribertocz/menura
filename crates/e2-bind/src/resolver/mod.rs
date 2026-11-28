//! Row 3 of E2-5 for one name: it resolves against the model (E2-3.1, E2-3.2, E2-3.5) and is
//! authorized as soon as it resolves (E2-4). A name that does not resolve and one that is not
//! authorized give the same answer, `None` (E2-4.4).

mod candidates;
mod relation;

use std::collections::HashMap;

use menura_contracts::types::audit_record::Access;
use menura_model::expression::Reference;
use menura_model::internal_model::{DiagnosticCode, FieldPath, InternalModel, NormalizedName};
use menura_representations::bound_query::{
    AuthorizationDecision, AuthorizedObject, AuthorizedSubgraph, MetricPath,
};
use menura_representations::query_tree::QueryReference;
use menura_security::{AdmittedSecurityContext, Authorization};

use crate::authorized_query::ResolvedName;
use crate::names::{identifier_name, reference_names};

pub(crate) use relation::{NameScope, Position, RelationScope};

/// The query error code E0 records for a relationship it drops (`E0E-01`, `E0E-02`).
const DROPPED_RELATIONSHIP: &str = "E_NAME_NOT_FOUND";

/// Resolves and authorizes names, and records each decision once, the first time an object
/// is used (E2-4.7).
pub(crate) struct Resolver<'model> {
    model: &'model InternalModel,
    authorization: Authorization<'model>,
    decided: HashMap<AuthorizedObject, Access>,
    decisions: Vec<AuthorizationDecision>,
}

impl<'model> Resolver<'model> {
    /// The resolver of the user of `context` against `model`.
    pub(crate) fn new(model: &'model InternalModel, context: &AdmittedSecurityContext) -> Self {
        Self {
            model,
            authorization: Authorization::new(model, context),
            decided: HashMap::new(),
            decisions: Vec::new(),
        }
    }

    /// The decisions taken, in order, and the authorized subgraph (E2-4.8).
    pub(crate) fn finish(self) -> (Vec<AuthorizationDecision>, AuthorizedSubgraph) {
        let subgraph = self.authorized_subgraph();
        (self.decisions, subgraph)
    }

    /// The authorized subgraph (E2-4.8): the datasets whose `access` admits the user, and the
    /// relationships between them whose `from_columns` and `to_columns` read only fields the
    /// user may see (E2-4.2). The query does not name these relationships, so checking their
    /// keys records no decision (E2-4.7).
    fn authorized_subgraph(&self) -> AuthorizedSubgraph {
        let admitted = self.authorization.subgraph();
        let relationships = admitted
            .relationships()
            .iter()
            .filter(|name| {
                self.model.relationship(name).is_some_and(|relationship| {
                    self.keys_readable(&relationship.from, &relationship.from_columns)
                        && self.keys_readable(&relationship.to, &relationship.to_columns)
                })
            })
            .cloned()
            .collect();
        AuthorizedSubgraph::new(admitted.datasets().to_vec(), relationships)
    }

    /// Whether the user may read every key column of `dataset`, without recording a decision:
    /// a field by its field restrictions, any other column by the restrictions that protect
    /// that source column (SCY-3.1, SCY-3.1a).
    fn keys_readable(&self, dataset: &NormalizedName, columns: &[NormalizedName]) -> bool {
        columns.iter().all(|column| {
            if self.is_field(dataset, column) {
                self.authorization
                    .field(&field_path(dataset, column))
                    .access
                    == Access::Granted
            } else {
                self.authorization.key_column(dataset, column)
            }
        })
    }

    /// Whether `column` is a field of `dataset`.
    fn is_field(&self, dataset: &NormalizedName, column: &NormalizedName) -> bool {
        self.model
            .dataset(dataset)
            .is_some_and(|found| found.field(column).is_some())
    }

    /// The field or metric a reference of the wide-table form names (QRY-3.2):
    /// `dataset.field`, `dataset.metric` or `metric`.
    pub(crate) fn reference(&mut self, reference: &QueryReference) -> Option<ResolvedName> {
        match reference_names(reference)?.as_slice() {
            [metric] => self.semantic_metric(metric),
            [dataset, member] => self.member(dataset, member),
            _ => None,
        }
    }

    /// The field or metric a reference of a filter expression names, in the same forms.
    pub(crate) fn expression_reference(&mut self, reference: &Reference) -> Option<ResolvedName> {
        let member = identifier_name(&reference.name);
        match &reference.qualifier {
            Some(dataset) => self.member(&identifier_name(dataset), &member),
            None => self.semantic_metric(&member),
        }
    }

    /// The dataset a one-part reference names, when the user may use it (E2-4.1).
    pub(crate) fn dataset_reference(
        &mut self,
        reference: &QueryReference,
    ) -> Option<NormalizedName> {
        match reference_names(reference)?.as_slice() {
            [dataset] if self.dataset(dataset) => Some(dataset.clone()),
            _ => None,
        }
    }

    /// The relationship a one-part reference names, when it is in the model and authorized
    /// (E2-4.8): E0 has not dropped it (`E0E-01`, `E0E-02`, E0-4 table), both ends are
    /// authorized datasets, and every key field it reads is authorized (E2-4.2).
    pub(crate) fn relationship_reference(
        &mut self,
        reference: &QueryReference,
    ) -> Option<NormalizedName> {
        let names = reference_names(reference)?;
        let [name] = names.as_slice() else {
            return None;
        };
        let relationship = self
            .model
            .relationship(name)
            .filter(|found| !is_dropped(&found.query_errors))?;
        let ends = [
            (&relationship.from, &relationship.from_columns),
            (&relationship.to, &relationship.to_columns),
        ];
        for (dataset, columns) in ends {
            if !self.dataset(dataset) || !self.key_fields(dataset, columns) {
                return None;
            }
        }
        Some(name.clone())
    }

    /// Whether every key column of `dataset` is authorized: a field by its decision, any other
    /// column, which reads that source column, by the restrictions that protect it (E2-4.2).
    fn key_fields(&mut self, dataset: &NormalizedName, columns: &[NormalizedName]) -> bool {
        columns.iter().all(|column| {
            if self.is_field(dataset, column) {
                self.field(&field_path(dataset, column))
            } else {
                self.authorization.key_column(dataset, column)
            }
        })
    }

    /// A metric of the semantic model, when it exists and is authorized.
    fn semantic_metric(&mut self, name: &NormalizedName) -> Option<ResolvedName> {
        self.model.metric(name)?;
        let path = MetricPath::SemanticModel(name.clone());
        self.metric(&path).then_some(ResolvedName::Metric(path))
    }

    /// A field or a dataset metric of `dataset`, when both exist and are authorized: the
    /// dataset first, then the member.
    fn member(
        &mut self,
        dataset: &NormalizedName,
        member: &NormalizedName,
    ) -> Option<ResolvedName> {
        let found = self.model.dataset(dataset)?;
        let is_field = found.field(member).is_some();
        let is_metric = found.metrics.iter().any(|metric| &metric.name == member);
        if !self.dataset(dataset) {
            return None;
        }
        if is_field {
            let path = field_path(dataset, member);
            return self.field(&path).then_some(ResolvedName::Field(path));
        }
        if is_metric {
            let path = MetricPath::Dataset {
                dataset: dataset.clone(),
                metric: member.clone(),
            };
            return self.metric(&path).then_some(ResolvedName::Metric(path));
        }
        None
    }

    /// Whether the dataset `name` exists and the user may use it.
    fn dataset(&mut self, name: &NormalizedName) -> bool {
        self.model.dataset(name).is_some()
            && self.decide(AuthorizedObject::Dataset(name.clone()), |authorization| {
                authorization.dataset(name)
            })
    }

    /// Whether the user may read the field `path`.
    fn field(&mut self, path: &FieldPath) -> bool {
        self.decide(AuthorizedObject::Field(path.clone()), |authorization| {
            authorization.field(path)
        })
    }

    /// Whether the user may use the metric `path`. The datasets it reads are used too
    /// (E2-4.1), so they are decided and recorded first, in name order.
    fn metric(&mut self, path: &MetricPath) -> bool {
        for dataset in self.authorization.metric_datasets(path) {
            self.dataset(&dataset);
        }
        self.decide(AuthorizedObject::Metric(path.clone()), |authorization| {
            authorization.metric(path)
        })
    }

    /// The decision on `object`: taken and recorded the first time, reused afterwards.
    fn decide(
        &mut self,
        object: AuthorizedObject,
        take: impl FnOnce(&Authorization<'model>) -> AuthorizationDecision,
    ) -> bool {
        if let Some(access) = self.decided.get(&object) {
            return *access == Access::Granted;
        }
        let decision = take(&self.authorization);
        let access = decision.access;
        self.decided.insert(object, access);
        self.decisions.push(decision);
        access == Access::Granted
    }
}

/// The field `field` of `dataset`.
fn field_path(dataset: &NormalizedName, field: &NormalizedName) -> FieldPath {
    FieldPath {
        dataset: dataset.clone(),
        field: field.clone(),
    }
}

/// Whether E0 dropped the relationship whose query errors are `query_errors`: it records
/// `E_NAME_NOT_FOUND` for a relationship it leaves out of the model (E0-4 table, `W0E-04`).
fn is_dropped(query_errors: &[DiagnosticCode]) -> bool {
    query_errors
        .iter()
        .any(|code| code.as_str() == DROPPED_RELATIONSHIP)
}
