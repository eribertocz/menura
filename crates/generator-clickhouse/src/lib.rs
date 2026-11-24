//! Stage E6: generates `ClickHouse` SQL from an optimized plan (E6-7).
//!
//! [`generate_sql`] writes one read-only statement with one common table expression per plan
//! node, in plan order (E6-4.1, E6-4.4), binds every query and security value as a typed
//! parameter `{pn:<type>}` (E6-4.2, E6-7.3), ends with the `SETTINGS` clause of E6-7.8 and
//! builds the result schema (PLN-1) with the `ClickHouse` type names of E6-7.2. All
//! `ClickHouse` knowledge lives here (E6-3.2). [`generate_check_sql`] writes the query of an
//! integrity check from its logical plan with the same steps (CHK-4.2, E6-7.11).
//!
//! Before any SQL is written, a plan that scans a relation `source` of three or more parts
//! stops with `E6-01` (E6-7.1); then a plan that reads or returns a `time`, an `interval` or a
//! `timestamp` without time zone stops with `E6-01` (E6-7.12).
//!
//! The `SETTINGS` clause fixes these settings, the full list of E6-7.8:
//!
//! | Setting | Value |
//! |---|---|
//! | `join_use_nulls` | `1` |
//! | `transform_null_in` | `0` |
//! | `prefer_column_name_to_alias` | `1` |
//! | `enable_positional_arguments` | `1` |
//! | `short_circuit_function_evaluation` | `'force_enable'` |
//! | `enable_extended_results_for_datetime_functions` | `1` |
//! | `aggregate_functions_null_for_empty` | `1` |
//! | `decimal_check_overflow` | `1` |
//! | `use_variant_as_common_type` | `0` |
//!
//! The reason for each is documented with the clause, in the `settings` module.

#![forbid(unsafe_code)]

mod additivity_error;
mod body;
mod check;
mod day;
mod deduplicate;
mod error;
mod expressible;
mod expression;
mod full_join;
mod identifiers;
mod join;
mod join_keys;
mod lookup;
mod null_safe;
mod ordering;
mod output;
mod parameters;
mod period_range;
mod periods;
mod population;
mod raise;
mod result_schema;
mod scan;
mod semi_join;
mod settings;
mod shifted_filter;
mod snapshot;
mod source_columns;
mod steps;
mod stitch;
mod to_date;
mod types;
mod values;

pub use check::{
    CheckInputs, GeneratedCheckSql, PeriodicCheck, SnapshotCheck, SnapshotTime, ValidityCheck,
    generate_check_sql, generate_periodic_check_sql, generate_snapshot_check_sql,
    generate_validity_check_sql,
};
pub use error::{GenerateError, InvariantViolation};
pub use output::GeneratedSql;

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::PlanNode;
use menura_security::AdmittedSecurityContext;

use crate::body::{SelectBody, statement};
use crate::expressible::check_query;
use crate::expression::Environment;
use crate::steps::{StepWriter, pass_through};

/// Generates the `ClickHouse` SQL of `plan`, the optimized plan of `query`, over `model` and its
/// admitted `catalog` schema, for the user of `security`.
///
/// The plan follows the 0.3.0 conventions: a `Scan` projects fields of its dataset by name,
/// every other node reads the columns of its input by unqualified reference, the literals of a
/// node from the query are query values, and those of a node from the model are constants.
///
/// # Errors
///
/// [`GenerateError::InexpressibleSource`] (`E6-01`) when the plan scans a relation `source` of
/// three or more parts (E6-7.1), then [`GenerateError::InexpressibleType`] (`E6-01`) when it
/// reads or returns a value of a type `ClickHouse` cannot express (E6-7.12), both checked
/// before any SQL is written; otherwise a [`GenerateError::Invariant`] when the input breaks
/// the contract of the stages before E6, such as a node outside the 0.3.0 slice (VS-1) or a
/// `source` without a catalog relation.
pub fn generate_sql(
    plan: &PlanNode,
    catalog: &CatalogSchemaDocument,
    model: &InternalModel,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
) -> Result<GeneratedSql, GenerateError> {
    check_query(plan, query, model, catalog)?;
    let environment = Environment {
        catalog,
        model,
        security: Some(security),
        time: Some(query.time()),
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
