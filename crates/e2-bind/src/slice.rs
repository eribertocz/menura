//! Row 12c of E2-5: the 0.3.0 slice (VS-1). The first element in query order that needs a
//! capability outside the slice is `E_NOT_YET_SUPPORTED`, located at that element, and the
//! message names the capability whose version comes first (E2-5.4, VS-2).

use std::collections::{BTreeMap, BTreeSet};

use menura_model::internal_model::{DeclaredAdditivity, InternalModel, Metric, NormalizedName};

use crate::authorized_query::ResolvedName;
use crate::error::{BindCode, BindDiagnostic};
use crate::model_elements::metric;
use crate::named::{NamedElement, NamedTarget};
use crate::read_metrics::metric_closure;

/// A capability outside the 0.3.0 slice. The order of the variants is the order of table 7.1,
/// then the order in which VS-1 lists the capabilities of one version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Capability {
    SeveralDatasets,
    Paths,
    DerivedMetric,
    RowPolicyPropagation,
    Additivity,
    Population,
    ContextMetric,
}

impl Capability {
    /// The capability as a message names it.
    fn description(self) -> &'static str {
        match self {
            Self::SeveralDatasets => {
                "metrics, dimensions, filters or time settings on more than one dataset"
            }
            Self::Paths => "`paths`",
            Self::DerivedMetric => "derived metrics",
            Self::RowPolicyPropagation => "row policies propagated from related datasets",
            Self::Additivity => "metrics with an additivity declaration",
            Self::Population => "metrics with a population",
            Self::ContextMetric => "context metrics",
        }
    }

    /// The version that adds it (table 7.1).
    fn version(self) -> &'static str {
        match self {
            Self::SeveralDatasets
            | Self::Paths
            | Self::DerivedMetric
            | Self::RowPolicyPropagation => "0.4.0",
            Self::Additivity | Self::Population => "0.5.0",
            Self::ContextMetric => "0.8.0",
        }
    }
}

/// Row 12c over `named`, in query order. The home dataset is the dataset of the first element
/// that reads one; an element that reads another dataset needs several datasets.
pub(crate) fn check_slice(
    model: &InternalModel,
    named: &[NamedElement],
) -> Result<(), BindDiagnostic> {
    let mut home: Option<&NormalizedName> = None;
    for element in named {
        let ElementNeeds {
            datasets,
            capabilities,
        } = element_needs(model, &element.target);
        let mut needed: BTreeSet<Capability> = capabilities.into_iter().collect();
        match (home, datasets.first()) {
            (None, Some(first)) => {
                home = Some(first);
                if reaches_row_policy(model, first) {
                    needed.insert(Capability::RowPolicyPropagation);
                }
            }
            (Some(home), _) if datasets.iter().any(|dataset| *dataset != home) => {
                needed.insert(Capability::SeveralDatasets);
            }
            _ => {}
        }
        if let Some(capability) = needed.first() {
            return Err(not_yet_supported(element, *capability));
        }
    }
    Ok(())
}

/// What one element reads and needs.
struct ElementNeeds<'model> {
    /// The datasets it reads, ordered by name.
    datasets: Vec<&'model NormalizedName>,
    /// The capabilities it needs by itself.
    capabilities: Vec<Capability>,
}

/// What `target` reads and needs: a field reads its dataset; a metric reads the home datasets
/// of the metric and of every metric it reads, and needs the capabilities of each of them
/// (E2-5.4); `paths` entries need paths.
fn element_needs<'model>(
    model: &'model InternalModel,
    target: &'model NamedTarget,
) -> ElementNeeds<'model> {
    match target {
        NamedTarget::Member(ResolvedName::Field(path)) => ElementNeeds {
            datasets: vec![&path.dataset],
            capabilities: Vec::new(),
        },
        NamedTarget::Member(ResolvedName::Metric(path)) => match metric(model, path) {
            Some(found) => metric_needs(model, found),
            None => ElementNeeds {
                datasets: Vec::new(),
                capabilities: Vec::new(),
            },
        },
        NamedTarget::Dataset(_) | NamedTarget::Relationship(_) => ElementNeeds {
            datasets: Vec::new(),
            capabilities: vec![Capability::Paths],
        },
    }
}

/// What `root` needs, over it and every metric it reads: their home datasets, several datasets
/// when those are more than one, and the capabilities of their extensions.
fn metric_needs<'model>(
    model: &'model InternalModel,
    root: &'model Metric,
) -> ElementNeeds<'model> {
    let closure = metric_closure(model, root);
    let datasets: BTreeSet<&NormalizedName> = closure
        .iter()
        .filter_map(|read| read.classification.as_ref())
        .flat_map(|classification| &classification.home_datasets)
        .collect();
    let mut capabilities: Vec<Capability> = closure
        .into_iter()
        .flat_map(extension_capabilities)
        .collect();
    if datasets.len() > 1 {
        capabilities.push(Capability::SeveralDatasets);
    }
    ElementNeeds {
        datasets: datasets.into_iter().collect(),
        capabilities,
    }
}

/// The capabilities the extensions of one metric need.
fn extension_capabilities(metric: &Metric) -> Vec<Capability> {
    let mut needed = Vec::new();
    if is_derived(metric) {
        needed.push(Capability::DerivedMetric);
    }
    match &metric.declared_additivity {
        Some(DeclaredAdditivity::SemiAdditive(semi_additive)) => {
            needed.push(Capability::Additivity);
            if semi_additive.population.is_some() {
                needed.push(Capability::Population);
            }
        }
        Some(DeclaredAdditivity::NonAdditive) => needed.push(Capability::Additivity),
        None => {}
    }
    if metric.context.is_some() {
        needed.push(Capability::ContextMetric);
    }
    needed
}

/// Whether `metric` is a derived metric: it has the `derived` extension, or it reads metrics
/// with `AGG(…)` without being a context metric, as a development-line metric that reads
/// `AGG(dataset.metric)` does (DER-2.2).
fn is_derived(metric: &Metric) -> bool {
    metric.derived.is_some()
        || (metric.context.is_none()
            && metric
                .classification
                .as_ref()
                .is_some_and(|classification| !classification.inputs.is_empty()))
}

/// Whether `home` reaches, through many-to-one steps of usable relationships, another dataset
/// that declares row policies (E4-6.5). Decided from the model alone, whatever the user's
/// roles (VS-1).
fn reaches_row_policy(model: &InternalModel, home: &NormalizedName) -> bool {
    let mut steps: BTreeMap<&NormalizedName, Vec<&NormalizedName>> = BTreeMap::new();
    for edge in &model.parts().graph.edges {
        steps.entry(&edge.from).or_default().push(&edge.to);
    }
    let mut reached = BTreeSet::from([home]);
    let mut pending = vec![home];
    while let Some(dataset) = pending.pop() {
        for next in steps.get(dataset).into_iter().flatten() {
            if reached.insert(next) {
                pending.push(next);
            }
        }
    }
    reached
        .into_iter()
        .filter(|dataset| *dataset != home)
        .any(|dataset| {
            model
                .dataset(dataset)
                .is_some_and(|found| !found.row_policies.is_empty())
        })
}

fn not_yet_supported(element: &NamedElement, capability: Capability) -> BindDiagnostic {
    BindDiagnostic::new(
        BindCode::NotYetSupported,
        format!(
            "this query needs {}, which is not supported before {}",
            capability.description(),
            capability.version()
        ),
        &element.origin,
    )
}
