//! The SQL of an integrity check (CHK-4.2): a logical plan generated like the plan of a query,
//! with no security context and no query time settings, because no user runs it (CHK-2.2).

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::descriptions::CheckParameter;
use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::{Operation, PlanNode};

use crate::body::{SelectBody, statement};
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Environment;
use crate::steps::{StepWriter, error, pass_through};

/// The SQL of an integrity-check plan and the parameters it binds (CHK-2, CHK-7.1).
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedCheckSql {
    /// One read-only statement (E6-4.1) that returns one row per violation (CHK-2.1).
    pub sql: String,
    /// Its parameters by position (`$1`, `$2`, …), in order of first appearance in `sql`
    /// (E6-4.2): each a value the compiler fixed or a slot the pipeline binds (CHK-7.1).
    pub parameters: Vec<CheckParameter>,
}

/// Generates the `DuckDB` SQL of the integrity-check `plan` over `model` and its admitted
/// `catalog` schema (CHK-4.2). The nodes of a check come from the model, so its constants are
/// written as literals (E6-4.11). The statement selects every column of `plan`, in order.
///
/// # Errors
///
/// A [`GenerateError`] when the plan breaks the contract of the generator, or when it needs a
/// value only a query has: a security attribute or the reference time. The time zone of a
/// check is its `time_zone` slot (CHK-7.3), and the bounds of a `PeriodRange` its
/// `period_from` and `period_to` slots (CHK-7.4).
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
        parameters: writer.parameters.into_check_list(),
    })
}

impl<'input> StepWriter<'input> {
    /// The body and kind of a node that only integrity checks have: `PeriodRange` and
    /// `FullJoin` (CHK-6.6, CHK-7.4).
    pub(crate) fn check_step(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<(SelectBody, &'static str), GenerateError> {
        match node.operation() {
            Operation::PeriodRange { input, coverage } => {
                Ok((self.period_range(node, input, coverage)?, "periodrange"))
            }
            Operation::FullJoin {
                left,
                right,
                on,
                right_prefix,
            } => Ok((
                self.full_join(node, (left, right), on, right_prefix)?,
                "fulljoin",
            )),
            _ => Err(error(InvariantViolation::NodeOutsideSlice, node)),
        }
    }
}
