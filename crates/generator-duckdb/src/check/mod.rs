//! The SQL of an integrity check (CHK-4.2): a logical plan generated like the plan of a query,
//! with no security context and no query time settings, because no user runs it (CHK-2.2).
//! The checks that compare rows with other rows of the same relation, or with a series of
//! periods (CHK-6.3–CHK-6.5), write their last steps over the step of their rows' plan.

mod periodic;
mod shapes;
mod snapshot;
mod sql;
mod validity;

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::Parameter;
use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::PlanNode;

use crate::body::{SelectBody, statement};
use crate::error::GenerateError;
use crate::expression::Environment;
use crate::parameters::SharedValue;
use crate::steps::{StepWriter, pass_through};

pub use shapes::{PeriodicCheck, SnapshotCheck, SnapshotTime, ValidityCheck};

/// The SQL of an integrity-check plan and the parameters it binds (CHK-2, CHK-7.1).
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedCheckSql {
    /// One read-only statement (E6-4.1) that returns one row per violation (CHK-2.1).
    pub sql: String,
    /// The parameters by position (`$1`, `$2`, …), in order of first appearance in `sql`
    /// (E6-4.2).
    pub parameters: Vec<GeneratedCheckParameter>,
}

/// One parameter of a check statement (CHK-7.1).
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratedCheckParameter {
    /// A value the compiler fixed.
    Fixed(Parameter),
    /// The `time_zone` slot at `position`, which the pipeline fills (CHK-7.3).
    TimeZone {
        /// Its position, from 1.
        position: u64,
    },
}

/// What a check statement may read: the catalog schema and the model.
#[derive(Debug, Clone, Copy)]
pub struct CheckInputs<'input> {
    /// The admitted catalog schema.
    pub catalog: &'input CatalogSchemaDocument,
    /// The internal model.
    pub model: &'input InternalModel,
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
    let environment = environment(CheckInputs { catalog, model }, false);
    let mut writer = StepWriter::new(&environment);
    let written = writer.write(plan)?;
    let last = SelectBody {
        items: pass_through(plan, plan.schema())?,
        from: written.name,
        clauses: written.ordering.into_iter().collect(),
    };
    Ok(finish(writer, &last))
}

/// Generates the `DuckDB` SQL of the `population` check with `mode: validity` (CHK-6.3). A
/// `timestamp with time zone` field reads its day in the `time_zone` slot (CHK-7.3).
///
/// # Errors
///
/// A [`GenerateError`] when a column of `check` is not a column of its rows, or when the plan
/// of its rows breaks the contract of the generator.
pub fn generate_validity_check_sql(
    check: &ValidityCheck<'_>,
    inputs: CheckInputs<'_>,
) -> Result<GeneratedCheckSql, GenerateError> {
    let environment = environment(inputs, true);
    let mut writer = StepWriter::new(&environment);
    let last = writer.validity_check(check)?;
    Ok(finish(writer, &last))
}

/// Generates the `DuckDB` SQL of the `population` check with `mode: periodic` (CHK-6.4). A
/// `timestamp with time zone` period reads its day in the `time_zone` slot (CHK-7.3).
///
/// # Errors
///
/// As [`generate_validity_check_sql`].
pub fn generate_periodic_check_sql(
    check: &PeriodicCheck<'_>,
    inputs: CheckInputs<'_>,
) -> Result<GeneratedCheckSql, GenerateError> {
    let environment = environment(inputs, true);
    let mut writer = StepWriter::new(&environment);
    let last = writer.periodic_check(check)?;
    Ok(finish(writer, &last))
}

/// Generates the `DuckDB` SQL of the `semi_additive` check (CHK-6.5, ADD-6). A `timestamp
/// with time zone` `over`, or time field of the population, reads its day in the `time_zone`
/// slot (CHK-7.3).
///
/// # Errors
///
/// As [`generate_validity_check_sql`].
pub fn generate_snapshot_check_sql(
    check: &SnapshotCheck<'_>,
    inputs: CheckInputs<'_>,
) -> Result<GeneratedCheckSql, GenerateError> {
    let environment = environment(inputs, true);
    let mut writer = StepWriter::new(&environment);
    let last = writer.snapshot_check(check)?;
    Ok(finish(writer, &last))
}

fn environment(inputs: CheckInputs<'_>, time_zone_slot: bool) -> Environment<'_> {
    Environment {
        catalog: inputs.catalog,
        model: inputs.model,
        security: None,
        time: None,
        time_zone_slot,
    }
}

/// The statement of `writer`'s steps and `last`, with its parameters.
fn finish(writer: StepWriter<'_>, last: &SelectBody) -> GeneratedCheckSql {
    let sql = statement(&writer.steps, last);
    let time_zone = writer.parameters.shared_position(&SharedValue::TimeZone);
    let parameters = writer
        .parameters
        .into_list()
        .into_iter()
        .map(|parameter| {
            if Some(parameter.position) == time_zone {
                GeneratedCheckParameter::TimeZone {
                    position: parameter.position,
                }
            } else {
                GeneratedCheckParameter::Fixed(parameter)
            }
        })
        .collect();
    GeneratedCheckSql { sql, parameters }
}
