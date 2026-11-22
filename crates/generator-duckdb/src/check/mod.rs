//! The SQL of an integrity check (CHK-4.2): a logical plan generated like the plan of a query,
//! with no security context and no query time settings, because no user runs it (CHK-2.2).

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::Parameter;
use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::PlanNode;

use crate::body::{SelectBody, statement};
use crate::error::GenerateError;
use crate::expression::Environment;
use crate::steps::{StepWriter, pass_through};

/// The SQL of an integrity-check plan and the parameters it binds (CHK-2, CHK-7.1).
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedCheckSql {
    /// One read-only statement (E6-4.1) that returns one row per violation (CHK-2.1).
    pub sql: String,
    /// The parameters with a value the compiler fixed, by position (`$1`, `$2`, …), in order
    /// of first appearance in `sql` (E6-4.2).
    pub parameters: Vec<Parameter>,
}

/// Generates the `DuckDB` SQL of the integrity-check `plan` over `model` and its admitted
/// `catalog` schema (CHK-4.2). The nodes of a check come from the model, so its constants are
/// written as literals (E6-4.11). The statement selects every column of `plan`, in order.
///
/// # Errors
///
/// A [`GenerateError`] when the plan breaks the contract of the generator, or when it needs a
/// value only a query has: a security attribute, the reference time or the query time zone.
pub fn generate_check_sql(
    plan: &PlanNode,
    catalog: &CatalogSchemaDocument,
    model: &InternalModel,
) -> Result<GeneratedCheckSql, GenerateError> {
    let environment = Environment {
        catalog,
        model,
        security: None,
        time: None,
    };
    let mut writer = StepWriter::new(&environment);
    let written = writer.write(plan)?;
    let last = SelectBody {
        items: pass_through(plan, plan.schema())?,
        from: written.name,
        clauses: written.ordering.into_iter().collect(),
    };
    Ok(GeneratedCheckSql {
        sql: statement(&writer.steps, &last),
        parameters: writer.parameters.into_list(),
    })
}
