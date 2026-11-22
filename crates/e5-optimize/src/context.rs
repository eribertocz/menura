//! What every rewrite reads besides the plan: the query, the model, the catalog and the
//! security context.

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundQuery;
use menura_security::AdmittedSecurityContext;

use crate::evidence::Evidence;

/// The inputs of E5 other than the plan (E5-2), shared by every rewrite.
#[derive(Debug, Clone, Copy)]
#[expect(
    dead_code,
    reason = "read by the rewrite bodies, filled in by their own issues"
)]
pub(crate) struct RewriteContext<'a> {
    /// The bound query the plan answers.
    pub(crate) query: &'a BoundQuery,
    /// The internal model.
    pub(crate) model: &'a InternalModel,
    /// The catalog schema the model was loaded against.
    pub(crate) schema: &'a CatalogSchemaDocument,
    /// The check evidence of the catalog state (CAT-7.3).
    pub(crate) evidence: Evidence<'a>,
    /// The admitted security context.
    pub(crate) security: &'a AdmittedSecurityContext,
}
