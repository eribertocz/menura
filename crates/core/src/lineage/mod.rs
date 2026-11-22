//! The lineage of a compiled plan (PLN-4): one entry per output column, with the model
//! elements, catalog columns, summary and row policies behind it.

mod metrics;
mod names;
pub(crate) mod policies;
mod provenance;
pub(crate) mod references;
mod scan;
mod semi_additive;
mod walk;

use std::collections::BTreeSet;

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::compiled_plan::{Lineage, PhysicalInput};
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::{BoundDimension, BoundMetric, BoundQuery};
use menura_representations::columns::OutputColumn;
use menura_representations::logical_plan::PlanNode;
use menura_security::AdmittedSecurityContext;

use crate::compile::InternalError;
use metrics::metrics_read;
use names::{model_inputs, written_name};
use policies::applied_policies;
use provenance::{NodeLineage, Provenance};
use scan::Sources;

/// What the lineage of a plan is computed against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LineageInputs<'input> {
    /// The model.
    pub(crate) model: &'input InternalModel,
    /// The admitted catalog schema.
    pub(crate) catalog: &'input CatalogSchemaDocument,
    /// The admitted security context of the user.
    pub(crate) context: &'input AdmittedSecurityContext,
}

/// The lineage of `plan`, the final optimized plan of `query` (PLN-4.3): one entry per output
/// column, in output order, named after the column.
///
/// # Errors
///
/// An [`InternalError`] when the plan names an element its inputs do not have, or does not
/// output one column per query column.
pub(crate) fn lineage(
    plan: &PlanNode,
    query: &BoundQuery,
    inputs: LineageInputs<'_>,
) -> Result<Vec<Lineage>, InternalError> {
    let sources = Sources {
        model: inputs.model,
        catalog: inputs.catalog,
    };
    let root = sources.node(plan, None)?;
    let columns = plan.schema().columns();
    if columns.len() != query.columns().iter().count() {
        return Err(InternalError::new(
            "core",
            "the plan does not output one column per query column",
        ));
    }
    columns
        .iter()
        .zip(query.columns().iter())
        .map(|(column, output)| entry(&root, column.name.as_str(), output, inputs))
        .collect()
}

/// Every row policy applied to the rows of `plan`, the final optimized plan, propagated
/// policies included, written and ordered as in PLN-4.7 (PLN-6.6): those of every branch.
///
/// # Errors
///
/// An [`InternalError`] when the plan names an element its inputs do not have.
pub(crate) fn row_policies(
    plan: &PlanNode,
    inputs: LineageInputs<'_>,
) -> Result<Vec<String>, InternalError> {
    let sources = Sources {
        model: inputs.model,
        catalog: inputs.catalog,
    };
    let root = sources.node(plan, None)?;
    let mut reads = root.shared;
    for branch in root.branches {
        reads.absorb(branch);
    }
    Ok(applied_policies(
        &reads.datasets,
        inputs.model,
        inputs.context,
    ))
}

/// The positions of the branches behind the output column `output` of provenance
/// `provenance` (PLN-4.8): for a dimension, every branch of the plan, those of its context
/// queries included; for a metric, every branch that computes one of its components.
fn behind(
    root: &NodeLineage,
    provenance: &Provenance,
    output: &OutputColumn<BoundDimension, BoundMetric>,
) -> Vec<usize> {
    match output {
        OutputColumn::Dimension(_) => (0..root.branches.len()).collect(),
        OutputColumn::Metric(_) => provenance.branches.iter().copied().collect(),
    }
}

/// The lineage entry of the output column `name`, the query column `output`, over the
/// lineage `root` of the final plan node.
fn entry(
    root: &NodeLineage,
    name: &str,
    output: &OutputColumn<BoundDimension, BoundMetric>,
    inputs: LineageInputs<'_>,
) -> Result<Lineage, InternalError> {
    let provenance = root.columns.get(name).ok_or_else(|| {
        InternalError::new("core", format!("the plan outputs `{name}` underived"))
    })?;
    let mut reads = root.shared.clone();
    for branch in behind(root, provenance, output) {
        if let Some(branch_reads) = root.branches.get(branch) {
            reads.absorb(branch_reads.clone());
        }
    }
    let metrics = match output {
        OutputColumn::Dimension(_) => BTreeSet::new(),
        OutputColumn::Metric(metric) => metrics_read(metric, inputs.model)?,
    };
    let physical_inputs = provenance
        .catalog_columns
        .iter()
        .chain(&reads.catalog_columns)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|read| PhysicalInput {
            source: read.source.clone(),
            column: read.column.clone(),
        })
        .collect();
    Ok(Lineage {
        output: name.to_owned(),
        model_inputs: model_inputs(&provenance.fields, &metrics, &provenance.relationships),
        physical_inputs,
        summary: reads
            .summary
            .as_ref()
            .map(|summary| written_name(summary).to_owned()),
        security_policies: applied_policies(&reads.datasets, inputs.model, inputs.context),
    })
}
