//! The SQL of an integrity check (CHK-4.2): a logical plan generated like the plan of a query,
//! with no security context and no query time settings, because no user runs it (CHK-2.2).
//! The checks that compare rows with other rows of the same relation, or with a series of
//! periods (CHK-6.3–CHK-6.5), or with the versions of another relation (VAL-2.16a), write
//! their last steps over the steps of their rows' plans.

mod periodic;
mod snapshot;
mod sql;
mod temporal;
mod validity;

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::LogicalType;
use menura_contracts::types::descriptions::CheckParameter;
use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::names::ColumnName;

use crate::body::{SelectBody, statement};
use crate::error::{GenerateError, InvariantViolation};
use crate::expressible::{check_model_plans, check_open_end, check_rule_open_end};
use crate::expression::Environment;
use crate::steps::{StepWriter, error, pass_through};

pub use menura_representations::check_shapes::{
    PeriodicCheck, SnapshotCheck, SnapshotTime, TemporalOrphansCheck, ValidityCheck, VersionRange,
};

/// The SQL of an integrity-check plan and the parameters it binds (CHK-2, CHK-7.1).
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedCheckSql {
    /// One read-only statement (E6-4.1) that returns one row per violation (CHK-2.1).
    pub sql: String,
    /// Its parameters by position (`$1`, `$2`, …), in order of first appearance in `sql`
    /// (E6-4.2): each a value the compiler fixed or a slot the pipeline binds (CHK-7.1).
    pub parameters: Vec<CheckParameter>,
}

/// What a check statement may read: the catalog schema and the model.
#[derive(Debug, Clone, Copy)]
pub struct CheckInputs<'input> {
    /// The admitted catalog schema.
    pub catalog: &'input CatalogSchemaDocument,
    /// The internal model.
    pub model: &'input InternalModel,
}

/// Generates the `ClickHouse` SQL of the integrity-check `plan` over `model` and its admitted
/// `catalog` schema (CHK-4.2). The nodes of a check come from the model, so its constants are
/// written as literals (E6-4.11). The statement selects every column of `plan`, in order.
///
/// # Errors
///
/// [`GenerateError::InexpressibleSource`] (`E6-01`) when the plan scans a relation `source` of
/// three or more parts (E6-7.1), then [`GenerateError::InexpressibleType`] (`E6-01`) when it
/// reads or returns a value of a type `ClickHouse` cannot express (E6-7.12); otherwise a
/// [`GenerateError`] when the plan breaks the contract of the generator, or when it needs a
/// value only a query has: a security attribute or the reference time. The time zone of a
/// check is its `time_zone` slot (CHK-7.3), and the bounds of a `PeriodRange` its
/// `period_from` and `period_to` slots (CHK-7.4).
pub fn generate_check_sql(
    plan: &PlanNode,
    catalog: &CatalogSchemaDocument,
    model: &InternalModel,
) -> Result<GeneratedCheckSql, GenerateError> {
    check_model_plans(&[plan], model, catalog)?;
    let environment = environment(CheckInputs { catalog, model });
    let mut writer = StepWriter::new(&environment);
    let written = writer.write(plan)?;
    let last = SelectBody {
        items: pass_through(plan, plan.schema())?,
        from: written.name,
        clauses: written.ordering.into_iter().collect(),
    };
    Ok(finish(writer, &last))
}

/// Generates the `ClickHouse` SQL of the `population` check with `mode: validity` (CHK-6.3). A
/// `timestamp with time zone` field reads its day in the `time_zone` slot (CHK-7.3).
///
/// # Errors
///
/// `E6-01` as [`generate_check_sql`] reports it for the plan of its rows; otherwise a
/// [`GenerateError`] when a column of `check` is not a column of its rows, or when the plan of
/// its rows breaks the contract of the generator.
pub fn generate_validity_check_sql(
    check: &ValidityCheck<'_>,
    inputs: CheckInputs<'_>,
) -> Result<GeneratedCheckSql, GenerateError> {
    check_model_plans(&[check.rows], inputs.model, inputs.catalog)?;
    if let Some(dataset) = scanned_dataset(check.rows) {
        check_open_end(
            check.open_end.as_ref(),
            column_type(check.rows, &check.valid_to),
            dataset,
            inputs.model,
        )?;
    }
    let environment = environment(inputs);
    let mut writer = StepWriter::new(&environment);
    let last = writer.validity_check(check)?;
    Ok(finish(writer, &last))
}

/// Generates the `ClickHouse` SQL of the `population` check with `mode: periodic` (CHK-6.4). A
/// `timestamp with time zone` period reads its day in the `time_zone` slot (CHK-7.3).
///
/// # Errors
///
/// As [`generate_validity_check_sql`].
pub fn generate_periodic_check_sql(
    check: &PeriodicCheck<'_>,
    inputs: CheckInputs<'_>,
) -> Result<GeneratedCheckSql, GenerateError> {
    check_model_plans(&[check.rows], inputs.model, inputs.catalog)?;
    let environment = environment(inputs);
    let mut writer = StepWriter::new(&environment);
    let last = writer.periodic_check(check)?;
    Ok(finish(writer, &last))
}

/// Generates the `ClickHouse` SQL of the `semi_additive` check (CHK-6.5, ADD-6). A `timestamp
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
    let mut plans = vec![check.rows];
    plans.extend(check.time.as_ref().map(|time| time.population));
    check_model_plans(&plans, inputs.model, inputs.catalog)?;
    if let Some(time) = &check.time
        && let Some(dataset) = scanned_dataset(time.population)
    {
        check_rule_open_end(time.rule, time.population.schema(), dataset, inputs.model)?;
    }
    let environment = environment(inputs);
    let mut writer = StepWriter::new(&environment);
    let last = writer.snapshot_check(check)?;
    Ok(finish(writer, &last))
}

/// Generates the `ClickHouse` SQL of the `temporal_orphans` check of a temporal relationship
/// (VAL-2.16a). It compares values and computes no day, so it has no `time_zone` slot
/// (CHK-7.3).
///
/// # Errors
///
/// As [`generate_validity_check_sql`].
pub fn generate_temporal_orphans_check_sql(
    check: &TemporalOrphansCheck<'_>,
    inputs: CheckInputs<'_>,
) -> Result<GeneratedCheckSql, GenerateError> {
    check_model_plans(&[check.rows, check.versions], inputs.model, inputs.catalog)?;
    if let Some(dataset) = scanned_dataset(check.versions) {
        check_open_end(
            check.range.open_end.as_ref(),
            column_type(check.versions, &check.range.valid_to),
            dataset,
            inputs.model,
        )?;
    }
    let environment = environment(inputs);
    let mut writer = StepWriter::new(&environment);
    let last = writer.temporal_orphans_check(check)?;
    Ok(finish(writer, &last))
}

/// The environment of a check: no security context and no query time settings.
fn environment(inputs: CheckInputs<'_>) -> Environment<'_> {
    Environment {
        catalog: inputs.catalog,
        model: inputs.model,
        security: None,
        time: None,
    }
}

/// The statement of `writer`'s steps and `last`, with its parameters.
fn finish(writer: StepWriter<'_>, last: &SelectBody) -> GeneratedCheckSql {
    GeneratedCheckSql {
        sql: statement(&writer.steps, last),
        parameters: writer.parameters.into_check_list(),
    }
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

/// The dataset of the `Scan` under the `Filter` nodes of `rows`, the plan of a check's rows:
/// the dataset that declares the population whose `open_end` the check reads (VAL-2.23).
fn scanned_dataset(rows: &PlanNode) -> Option<&NormalizedName> {
    let mut node = rows;
    loop {
        if let Operation::Scan { dataset, .. } = node.operation() {
            return Some(dataset);
        }
        node = node.inputs().into_iter().next()?;
    }
}

/// The type of the column `name` of `rows`, when it has one.
fn column_type<'plan>(rows: &'plan PlanNode, name: &ColumnName) -> Option<&'plan LogicalType> {
    rows.schema()
        .columns()
        .iter()
        .find(|column| &column.name == name)
        .map(|column| &column.logical_type)
}
