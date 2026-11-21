//! Stage E6: generates `DuckDB` SQL from an optimized plan.
//!
//! [`generate_sql`] writes one read-only statement with one common table expression per plan
//! node, in plan order (E6-4.1, E6-4.4), binds every query and security value as a positional
//! parameter (E6-4.2) and builds the result schema (PLN-1). All `DuckDB` knowledge lives here
//! (E6-3.2).

#![forbid(unsafe_code)]

mod body;
mod deduplicate;
mod error;
mod expression;
mod identifiers;
mod instant;
mod output;
mod parameters;
mod result_schema;
mod semi_join;
mod source_columns;
mod steps;
mod types;
mod values;

pub use error::{GenerateError, InvariantViolation};
pub use output::GeneratedSql;

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::PlanNode;
use menura_security::AdmittedSecurityContext;

use crate::body::{SelectBody, statement};
use crate::expression::Environment;
use crate::steps::{StepWriter, pass_through};

/// Generates the `DuckDB` SQL of `plan`, the optimized plan of `query`, over `model` and its
/// admitted `catalog` schema, for the user of `security`.
///
/// The plan follows the 0.3.0 conventions: a `Scan` projects fields of its dataset by name,
/// every other node reads the columns of its input by unqualified reference, the literals of a
/// node from the query are query values, and those of a node from the model are constants.
///
/// # Errors
///
/// A [`GenerateError`] when the input breaks the contract of the stages before E6, such as a
/// node outside the 0.3.0 slice (VS-1) or a `source` without a catalog relation.
pub fn generate_sql(
    plan: &PlanNode,
    catalog: &CatalogSchemaDocument,
    model: &InternalModel,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
) -> Result<GeneratedSql, GenerateError> {
    let environment = Environment {
        catalog,
        model,
        security,
        time: query.time(),
    };
    let mut writer = StepWriter::new(&environment);
    let written = writer.write(plan)?;
    let last = SelectBody {
        items: pass_through(plan, plan.schema())?,
        from: written.name,
        clauses: written.ordering.into_iter().collect(),
    };
    let result_schema = result_schema::result_schema(plan, query)?;
    Ok(GeneratedSql {
        sql: statement(&writer.steps, &last),
        parameters: writer.parameters.into_list(),
        result_schema,
    })
}
