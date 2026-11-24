//! The SQL of a plan in the target dialect (E6-3.1, E6-7): the one place of `core` that
//! chooses the generator, `generator-duckdb` or `generator-clickhouse`, for a dialect.

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::{Dialect, Parameter};
use menura_contracts::types::compiled_plan::ResultColumn;
use menura_contracts::types::descriptions::CheckParameter;
use menura_generator_clickhouse as clickhouse;
use menura_generator_duckdb as duckdb;
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundQuery;
use menura_representations::check_shapes::{PeriodicCheck, SnapshotCheck, ValidityCheck};
use menura_representations::logical_plan::PlanNode;
use menura_security::AdmittedSecurityContext;

use crate::compile::InternalError;
use crate::stop::Stop;

/// The SQL of a query plan, its bound parameters and its result schema (PLN-1).
pub(crate) struct QuerySql {
    /// One read-only statement in the dialect.
    pub(crate) sql: String,
    /// The bound parameters, by position.
    pub(crate) parameters: Vec<Parameter>,
    /// The output columns, in output order.
    pub(crate) result_schema: Vec<ResultColumn>,
}

/// The SQL of an integrity check and the parameters it binds (CHK-2, CHK-7.1).
pub(crate) struct CheckSql {
    /// One read-only statement in the dialect.
    pub(crate) sql: String,
    /// The parameters by position: values the compiler fixed and slots the pipeline binds.
    pub(crate) parameters: Vec<CheckParameter>,
}

/// What a generator reads besides the plan: the admitted catalog schema and the model.
#[derive(Clone, Copy)]
pub(crate) struct Sources<'input> {
    /// The admitted catalog schema.
    pub(crate) catalog: &'input CatalogSchemaDocument,
    /// The internal model.
    pub(crate) model: &'input InternalModel,
}

/// The shape of an integrity check, as both generators take it (CHK-4.2, CHK-6.3–CHK-6.5).
pub(crate) enum CheckShape<'plan> {
    /// A check that is one logical plan.
    Plan(&'plan PlanNode),
    /// The `population` check with `mode: validity`.
    Validity(ValidityCheck<'plan>),
    /// The `population` check with `mode: periodic`.
    Periodic(PeriodicCheck<'plan>),
    /// The `semi_additive` check.
    Snapshot(SnapshotCheck<'plan>),
}

/// The SQL of `plan`, the optimized plan of `query`, in `dialect`, for the user of `security`.
///
/// # Errors
///
/// `E6-01` when the `clickhouse` dialect cannot express a value of the plan (E6-7.12); an
/// [`InternalError`] of stage `E6` for any other error of the generator.
pub(crate) fn query_sql(
    dialect: Dialect,
    plan: &PlanNode,
    sources: Sources<'_>,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
) -> Result<QuerySql, Stop> {
    let Sources { catalog, model } = sources;
    match dialect {
        Dialect::Duckdb => duckdb::generate_sql(plan, catalog, model, query, security)
            .map(|generated| QuerySql {
                sql: generated.sql,
                parameters: generated.parameters,
                result_schema: generated.result_schema,
            })
            .map_err(|error| internal(&error).into()),
        Dialect::Clickhouse => clickhouse::generate_sql(plan, catalog, model, query, security)
            .map(|generated| QuerySql {
                sql: generated.sql,
                parameters: generated.parameters,
                result_schema: generated.result_schema,
            })
            .map_err(|error| clickhouse_stop(&error)),
    }
}

/// The SQL of the integrity check of `shape` in `dialect` (CHK-4.2, E6-7.11).
///
/// # Errors
///
/// As [`query_sql`].
pub(crate) fn check_sql(
    dialect: Dialect,
    shape: &CheckShape<'_>,
    sources: Sources<'_>,
) -> Result<CheckSql, Stop> {
    match dialect {
        Dialect::Duckdb => duckdb_check_sql(shape, sources)
            .map(|generated| CheckSql {
                sql: generated.sql,
                parameters: generated.parameters,
            })
            .map_err(|error| internal(&error).into()),
        Dialect::Clickhouse => clickhouse_check_sql(shape, sources)
            .map(|generated| CheckSql {
                sql: generated.sql,
                parameters: generated.parameters,
            })
            .map_err(|error| clickhouse_stop(&error)),
    }
}

fn duckdb_check_sql(
    shape: &CheckShape<'_>,
    sources: Sources<'_>,
) -> Result<duckdb::GeneratedCheckSql, duckdb::GenerateError> {
    let inputs = duckdb::CheckInputs {
        catalog: sources.catalog,
        model: sources.model,
    };
    match shape {
        CheckShape::Plan(plan) => duckdb::generate_check_sql(plan, sources.catalog, sources.model),
        CheckShape::Validity(check) => duckdb::generate_validity_check_sql(check, inputs),
        CheckShape::Periodic(check) => duckdb::generate_periodic_check_sql(check, inputs),
        CheckShape::Snapshot(check) => duckdb::generate_snapshot_check_sql(check, inputs),
    }
}

fn clickhouse_check_sql(
    shape: &CheckShape<'_>,
    sources: Sources<'_>,
) -> Result<clickhouse::GeneratedCheckSql, clickhouse::GenerateError> {
    let inputs = clickhouse::CheckInputs {
        catalog: sources.catalog,
        model: sources.model,
    };
    match shape {
        CheckShape::Plan(plan) => {
            clickhouse::generate_check_sql(plan, sources.catalog, sources.model)
        }
        CheckShape::Validity(check) => clickhouse::generate_validity_check_sql(check, inputs),
        CheckShape::Periodic(check) => clickhouse::generate_periodic_check_sql(check, inputs),
        CheckShape::Snapshot(check) => clickhouse::generate_snapshot_check_sql(check, inputs),
    }
}

/// The stop of a `clickhouse` generator error: `E6-01` as its diagnostic (E6-7.12), any other
/// error as a defect of E6.
fn clickhouse_stop(error: &clickhouse::GenerateError) -> Stop {
    match error.to_diagnostic() {
        Some(diagnostic) => diagnostic.into(),
        None => internal(error).into(),
    }
}

/// A generator error as a defect of E6 (PLN-5.1a).
fn internal(error: &impl std::fmt::Debug) -> InternalError {
    InternalError::new("E6", format!("{error:?}"))
}
