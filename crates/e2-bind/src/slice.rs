//! Row 12c of E2-5: the 0.4.0 slice (VS-1b). The first element in query order that needs a
//! capability outside the slice is `E_NOT_YET_SUPPORTED`, located at that element, and the
//! message names the capability whose version comes first (E2-5.4, VS-2). From 0.4.0 no
//! capability belongs to the whole query (E2-5.4a).

use menura_model::internal_model::{DeclaredAdditivity, InternalModel, Metric};

use crate::authorized_query::ResolvedName;
use crate::error::{BindCode, BindDiagnostic};
use crate::model_elements::metric;
use crate::named::{NamedElement, NamedTarget};
use crate::read_metrics::metric_closure;

/// A capability outside the 0.4.0 slice. The order of the variants is the order of table 7.1,
/// then the order in which E2-5.4a lists the capabilities of one version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Capability {
    Additivity,
    Population,
    ContextMetric,
}

impl Capability {
    /// The capability as a message names it.
    fn description(self) -> &'static str {
        match self {
            Self::Additivity => "metrics with an additivity declaration",
            Self::Population => "metrics with a population",
            Self::ContextMetric => "context metrics",
        }
    }

    /// The version that adds it (table 7.1).
    fn version(self) -> &'static str {
        match self {
            Self::Additivity | Self::Population => "0.5.0",
            Self::ContextMetric => "0.8.0",
        }
    }
}

/// Row 12c over `named`, in query order: the first element that needs a capability outside the
/// slice is reported at that element.
pub(crate) fn check_slice(
    model: &InternalModel,
    named: &[NamedElement],
) -> Result<(), BindDiagnostic> {
    for element in named {
        if let Some(capability) = first_needed_capability(model, &element.target) {
            return Err(not_yet_supported(element, capability));
        }
    }
    Ok(())
}

/// The first capability, in the order of [`Capability`], that `target` needs: a metric needs
/// the capabilities of its extensions and of every metric it reads (E2-5.4); fields, datasets
/// and relationships need none.
fn first_needed_capability(model: &InternalModel, target: &NamedTarget) -> Option<Capability> {
    let NamedTarget::Member(ResolvedName::Metric(path)) = target else {
        return None;
    };
    let root = metric(model, path)?;
    metric_closure(model, root)
        .into_iter()
        .flat_map(extension_capabilities)
        .min()
}

/// The capabilities the extensions of one metric need.
fn extension_capabilities(metric: &Metric) -> Vec<Capability> {
    let mut needed = Vec::new();
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
