//! Stage E5: rewrites the logical plan into an optimized plan.
//!
//! [`optimize`] runs the rewrites of E5-3 in order, each on the output of the previous one, and
//! returns the rewritten plan with the record of every rewrite it applied or rejected
//! (E5-2.2, E5-3.8). The output uses only the nodes of E4-3, so E6 needs no knowledge of which
//! rewrites ran.

#![forbid(unsafe_code)]

mod column_reads;
mod context;
mod evidence;
mod record;
mod rewrites;

#[cfg(test)]
mod evidence_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

use menura_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::PlanNode;
use menura_security::AdmittedSecurityContext;

use crate::context::RewriteContext;
use crate::evidence::Evidence;
use crate::rewrites::PIPELINE;

/// The output of E5: the optimized plan and the records of its rewrites (E5-2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizedPlan {
    plan: PlanNode,
    records: Vec<RewriteRecord>,
}

impl OptimizedPlan {
    /// The optimized plan: a tree of E4-3 nodes, equivalent to the input plan (E5-1).
    #[must_use]
    pub fn plan(&self) -> &PlanNode {
        &self.plan
    }

    /// Every rewrite applied, and every rewrite considered and rejected, in the order E5
    /// considered them (E5-3.8, E5-4.4). Before 0.7.0 they are not part of the compiled plan
    /// (VS-5).
    #[must_use]
    pub fn records(&self) -> &[RewriteRecord] {
        &self.records
    }

    /// The optimized plan, without the records.
    #[must_use]
    pub fn into_plan(self) -> PlanNode {
        self.plan
    }
}

/// Rewrites `plan`, the logical plan E4 built for `query`, into an equivalent plan that is
/// cheaper to run (E5-1), using the evidence of the catalog `state` (CAT-7.3) and the security
/// context `security`.
///
/// `state` is the admitted catalog state, or `None` when there is none or the handle ignored
/// it: E5 then applies only the rewrites that need no evidence (E5-2.1, CHK-4.1). The output
/// depends only on the content of the inputs, never on the order of their keys (E5-2.3).
///
/// E5 never fails: a rewrite that cannot be proven safe is not applied, and its record says
/// which condition failed (E5-3.8).
#[must_use]
pub fn optimize(
    plan: &PlanNode,
    query: &BoundQuery,
    model: &InternalModel,
    schema: &CatalogSchemaDocument,
    state: Option<&CatalogStateDocument>,
    security: &AdmittedSecurityContext,
) -> OptimizedPlan {
    let context = RewriteContext {
        query,
        model,
        schema,
        evidence: Evidence::new(model, state),
        security,
    };
    let mut records = Vec::new();
    let plan = PIPELINE.iter().fold(plan.clone(), |plan, (_, rewrite)| {
        rewrite(plan, &context, &mut records)
    });
    OptimizedPlan { plan, records }
}
