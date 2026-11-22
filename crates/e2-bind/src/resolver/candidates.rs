//! The candidates of PLN-5.4 step 2: the objects of one kind the user is authorized to see,
//! by their normalized names. Looking at a candidate records no decision (E2-4.7): only a name
//! the query uses is decided.

use menura_contracts::types::audit_record::Access;
use menura_model::internal_model::{Dataset, FieldPath, NormalizedName};
use menura_representations::bound_query::MetricPath;

use super::{NameScope, Resolver};
use crate::alternatives::{ExpectedKind, select_alternatives};
use crate::names::joined_names;

impl Resolver<'_> {
    /// The alternatives of a reference read in `scope` at a position that expects `kind`,
    /// whose parts normalize to `names` (PLN-5.4); empty when a part has no normalized form
    /// (`None`). Candidates come only from authorized objects (PLN-5.4a).
    pub(crate) fn alternatives(
        &self,
        kind: ExpectedKind,
        scope: NameScope<'_>,
        names: Option<Vec<NormalizedName>>,
    ) -> Vec<String> {
        let Some(names) = names else {
            return Vec::new();
        };
        let names = match kind {
            ExpectedKind::Field => scope.field_names(names),
            ExpectedKind::Metric | ExpectedKind::Dataset => names,
        };
        let reference = joined_names(&names);
        let candidates = match kind {
            ExpectedKind::Field => self.field_candidates(),
            ExpectedKind::Metric => self.metric_candidates(),
            ExpectedKind::Dataset => self
                .candidate_datasets()
                .map(|dataset| dataset.name.as_str().to_owned())
                .collect(),
        };
        select_alternatives(kind, &reference, candidates)
    }

    /// The datasets of the authorized subgraph (E2-4.8), without summaries (E2-4.6).
    fn candidate_datasets(&self) -> impl Iterator<Item = &Dataset> {
        let subgraph = self.authorization.subgraph();
        self.model
            .parts()
            .datasets
            .iter()
            .filter(|dataset| !dataset.summary)
            .filter(move |dataset| subgraph.datasets().binary_search(&dataset.name).is_ok())
    }

    /// The fields authorized by E2-4.2 on the datasets of the authorized subgraph, each as
    /// `<dataset>.<field>`.
    fn field_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();
        for dataset in self.candidate_datasets() {
            for field in &dataset.fields {
                let path = FieldPath {
                    dataset: dataset.name.clone(),
                    field: field.name.clone(),
                };
                if self.authorization.field(&path).access == Access::Granted {
                    candidates.push(qualified(&dataset.name, &field.name));
                }
            }
        }
        candidates
    }

    /// The metrics authorized by E2-4.3: those of the semantic model by their name, and those
    /// of a dataset of the authorized subgraph as `<dataset>.<metric>`.
    fn metric_candidates(&self) -> Vec<String> {
        let semantic = self.model.parts().metrics.iter().map(|metric| {
            (
                MetricPath::SemanticModel(metric.name.clone()),
                metric.name.as_str().to_owned(),
            )
        });
        let scoped = self.candidate_datasets().flat_map(|dataset| {
            dataset.metrics.iter().map(|metric| {
                (
                    MetricPath::Dataset {
                        dataset: dataset.name.clone(),
                        metric: metric.name.clone(),
                    },
                    qualified(&dataset.name, &metric.name),
                )
            })
        });
        semantic
            .chain(scoped)
            .filter(|(path, _)| self.authorization.metric(path).access == Access::Granted)
            .map(|(_, name)| name)
            .collect()
    }
}

/// `<dataset>.<member>`, from normalized names.
fn qualified(dataset: &NormalizedName, member: &NormalizedName) -> String {
    format!("{}.{}", dataset.as_str(), member.as_str())
}
