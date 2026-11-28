//! Row 3 of E2-5 in the SQL form: the `FROM` relation (QRY-5.1, E1-4.9) and the names read
//! through it (E1-4 "FROM", QRY-5.2, DEV-8).

use menura_model::expression::Reference;
use menura_model::internal_model::{FieldPath, NormalizedName};
use menura_representations::bound_query::MetricPath;
use menura_representations::query_tree::{QueryReference, QueryRelation};

use super::Resolver;
use crate::authorized_query::ResolvedName;
use crate::names::{identifier_name, reference_names};

/// The relation of a SQL-form query: the dataset it exposes, and the names that may qualify a
/// reference to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelationScope {
    dataset: NormalizedName,
    qualifiers: Vec<NormalizedName>,
}

impl RelationScope {
    /// The dataset the relation exposes.
    pub(crate) fn dataset(&self) -> &NormalizedName {
        &self.dataset
    }

    /// Whether `qualifier` names the relation: its dataset name or its alias (E1-4 "FROM").
    fn admits(&self, qualifier: &NormalizedName) -> bool {
        self.qualifiers.contains(qualifier)
    }
}

/// The position of a name in a SQL-form query, which decides what it names (E1-4.9, E0D-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Position {
    /// The argument of `AGG(…)` or `MEASURE(…)`: a metric exposed on the relation (QRY-5.2).
    Metric,
    /// Every other position: a field of the relation.
    Field,
}

/// Where the names of a query are looked up.
#[derive(Debug, Clone, Copy)]
pub(crate) enum NameScope<'scope> {
    /// The whole model: the wide-table form (QRY-3.2).
    Model,
    /// The one relation of a SQL-form query.
    Relation(&'scope RelationScope),
}

impl NameScope<'_> {
    /// The normalized names of a field reference as PLN-5.4 compares it: in the SQL form, a
    /// bare column, or one qualified by the relation's name or alias, becomes
    /// `<relation dataset>.<column>`, the reference it resolves to (E1-4 "FROM"). Any other
    /// reference is compared as written.
    pub(crate) fn field_names(self, mut names: Vec<NormalizedName>) -> Vec<NormalizedName> {
        let Self::Relation(relation) = self else {
            return names;
        };
        match names.as_mut_slice() {
            [column] => vec![relation.dataset.clone(), column.clone()],
            [qualifier, _] if relation.admits(qualifier) => {
                *qualifier = relation.dataset.clone();
                names
            }
            _ => names,
        }
    }
}

impl Resolver<'_> {
    /// The scope of `relation`, when its model is the model and its dataset exists and is
    /// authorized (QRY-5.1, E1-4.9). Names compare under E0D-1 (E2-3.1).
    pub(crate) fn relation(&mut self, relation: &QueryRelation) -> Option<RelationScope> {
        if identifier_name(&relation.model) != self.model.parts().name {
            return None;
        }
        let dataset = identifier_name(&relation.dataset);
        if !self.dataset(&dataset) {
            return None;
        }
        let mut qualifiers = vec![dataset.clone()];
        qualifiers.extend(relation.alias.as_ref().map(identifier_name));
        Some(RelationScope {
            dataset,
            qualifiers,
        })
    }

    /// The field or metric a reference of a column names in `scope`, at `position`.
    pub(crate) fn scoped_reference(
        &mut self,
        scope: NameScope<'_>,
        reference: &QueryReference,
        position: Position,
    ) -> Option<ResolvedName> {
        let NameScope::Relation(relation) = scope else {
            return self.reference(reference);
        };
        match reference_names(reference)?.as_slice() {
            [member] => self.relation_member(relation, None, member, position),
            [qualifier, member] => {
                self.relation_member(relation, Some(qualifier), member, position)
            }
            _ => None,
        }
    }

    /// The field or metric a reference of an expression names in `scope`, at `position`.
    pub(crate) fn scoped_expression_reference(
        &mut self,
        scope: NameScope<'_>,
        reference: &Reference,
        position: Position,
    ) -> Option<ResolvedName> {
        let NameScope::Relation(relation) = scope else {
            return self.expression_reference(reference);
        };
        let qualifier = reference.qualifier.as_ref().map(identifier_name);
        self.relation_member(
            relation,
            qualifier.as_ref(),
            &identifier_name(&reference.name),
            position,
        )
    }

    /// The member `member` of `relation`, unqualified or qualified by `qualifier`, looked up by
    /// its position (E1-4.9): at `Position::Metric` a metric exposed on the relation, else a
    /// field of it; at `Position::Field` a field of the relation, else a metric exposed on it.
    /// A candidate the user may not use is skipped exactly as if it did not exist, so the
    /// answer never reveals an unauthorized name (E2-4.4).
    fn relation_member(
        &mut self,
        relation: &RelationScope,
        qualifier: Option<&NormalizedName>,
        member: &NormalizedName,
        position: Position,
    ) -> Option<ResolvedName> {
        if qualifier.is_some_and(|name| !relation.admits(name)) {
            return None;
        }
        let field = self.relation_field(relation, member);
        let metrics = self.relation_metrics(relation, member);
        let candidates: Vec<ResolvedName> = match position {
            Position::Metric => metrics.into_iter().chain(field).collect(),
            Position::Field => field.into_iter().chain(metrics).collect(),
        };
        candidates
            .into_iter()
            .find_map(|candidate| self.authorized(candidate))
    }

    /// The field `member` of the relation's dataset, when it exists.
    fn relation_field(
        &self,
        relation: &RelationScope,
        member: &NormalizedName,
    ) -> Option<ResolvedName> {
        self.model.dataset(&relation.dataset)?.field(member)?;
        Some(ResolvedName::Field(FieldPath {
            dataset: relation.dataset.clone(),
            field: member.clone(),
        }))
    }

    /// The metrics named `member` exposed on the relation, in lookup order: a dataset metric of
    /// its dataset, then a metric of the semantic model exposed on it (QRY-5.2, DEV-8).
    fn relation_metrics(
        &self,
        relation: &RelationScope,
        member: &NormalizedName,
    ) -> Vec<ResolvedName> {
        let dataset_metric = self
            .model
            .dataset(&relation.dataset)
            .is_some_and(|dataset| dataset.metrics.iter().any(|metric| &metric.name == member))
            .then(|| {
                ResolvedName::Metric(MetricPath::Dataset {
                    dataset: relation.dataset.clone(),
                    metric: member.clone(),
                })
            });
        let model_metric = self
            .is_exposed(relation, member)
            .then(|| ResolvedName::Metric(MetricPath::SemanticModel(member.clone())));
        dataset_metric.into_iter().chain(model_metric).collect()
    }

    /// `found`, when the user may use it (E2-4.2, E2-4.3).
    fn authorized(&mut self, found: ResolvedName) -> Option<ResolvedName> {
        let granted = match &found {
            ResolvedName::Field(path) => self.field(path),
            ResolvedName::Metric(path) => self.metric(path),
        };
        granted.then_some(found)
    }

    /// Whether the metric of the semantic model `name` is exposed on `relation`: its home
    /// dataset is the relation's dataset alone (DEV-8). A metric E0 could not classify is
    /// exposed when E0 marked it unusable, so that row 12a reports it as in the wide-table
    /// form (P5).
    fn is_exposed(&self, relation: &RelationScope, name: &NormalizedName) -> bool {
        self.model
            .metric(name)
            .is_some_and(|metric| match &metric.classification {
                Some(classification) => {
                    classification.home_datasets.as_slice()
                        == std::slice::from_ref(&relation.dataset)
                }
                None => !metric.query_errors.is_empty(),
            })
    }
}
