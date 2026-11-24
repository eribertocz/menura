//! The queries of the `population` checks (CHK-6.3, CHK-6.4, POP-5): every row of the dataset
//! that declares the population, read by a `Scan` with no row policy (CHK-2.2), and the
//! columns of each role.

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::Dialect;
use menura_model::internal_model::{
    Dataset, InternalModel, NormalizedName, Population, PopulationMode, ValueType,
};
use menura_representations::check_shapes::{PeriodicCheck, ValidityCheck};
use menura_representations::logical_plan::{Membership, PlanNode, PopulationRule};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::key_columns::{STAGE, key_columns};
use super::plan_nodes::scan;
use super::result_names::added_column_name;
use crate::compile::InternalError;
use crate::dialect_sql::{CheckShape, CheckSql, Sources, check_sql};
use crate::stop::Stop;

/// The SQL of the `population` check of the population `target` in `dialect`, and the dataset
/// it reads.
pub(super) fn population_check_sql(
    dialect: Dialect,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
    target: &NormalizedName,
) -> Result<(CheckSql, NormalizedName), Stop> {
    let (dataset, population) = declared(model, target)?;
    let rows = population_rows(dataset, population, catalog)?;
    let entity = columns(&population.entity);
    let mut fields: Vec<String> = population
        .named_fields()
        .iter()
        .map(|name| name.as_str().to_owned())
        .collect();
    let shape = match &population.mode {
        PopulationMode::Validity {
            valid_from,
            valid_to,
            ..
        } => {
            fields.push(added_column_name("violation", &fields));
            let check = ValidityCheck {
                rows: &rows,
                entity,
                valid_from: ColumnName::new(valid_from.as_str()),
                valid_to: ColumnName::new(valid_to.as_str()),
                result_columns: fields,
            };
            CheckShape::Validity(check)
        }
        PopulationMode::Periodic { period, grain } => {
            fields.push(added_column_name("row_count", &fields));
            let check = PeriodicCheck {
                rows: &rows,
                entity,
                period: ColumnName::new(period.as_str()),
                grain: *grain,
                result_columns: fields,
            };
            CheckShape::Periodic(check)
        }
    };
    let generated = check_sql(dialect, &shape, Sources { catalog, model })?;
    Ok((generated, dataset.name.clone()))
}

/// The dataset that declares the population `name`, and the population.
pub(super) fn declared<'model>(
    model: &'model InternalModel,
    name: &NormalizedName,
) -> Result<(&'model Dataset, &'model Population), InternalError> {
    model
        .parts()
        .datasets
        .iter()
        .find_map(|dataset| {
            dataset
                .populations
                .iter()
                .find(|population| &population.name == name)
                .map(|population| (dataset, population))
        })
        .ok_or_else(|| {
            InternalError::new(
                STAGE,
                format!("no dataset declares the population {}", name.as_str()),
            )
        })
}

/// The `Scan` of every row of `dataset` that reads the fields of `population`, which is
/// what CHK-6.3 and CHK-6.4 read: they never apply `required_when`.
fn population_rows(
    dataset: &Dataset,
    population: &Population,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, InternalError> {
    let read: Vec<NormalizedName> = population.named_fields().into_iter().cloned().collect();
    rows_reading(dataset, population, &read, catalog)
}

/// The `Scan` of every row of `dataset` that reads the fields of `population` for its
/// expected pairs (POP-3); with a `required_when`, every typed field, since the condition may
/// read any of them.
pub(super) fn expected_population_rows(
    dataset: &Dataset,
    population: &Population,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, InternalError> {
    let mut read: Vec<NormalizedName> = population.named_fields().into_iter().cloned().collect();
    if population.required_when.is_some() {
        read.extend(
            dataset
                .fields
                .iter()
                .filter(|field| {
                    field
                        .typed
                        .as_ref()
                        .is_some_and(|typed| matches!(typed.value_type, ValueType::Logical(_)))
                })
                .map(|field| field.name.clone()),
        );
    }
    rows_reading(dataset, population, &read, catalog)
}

/// The `Scan` of every row of `dataset` that reads `read`, with no row policy (CHK-2.2).
fn rows_reading(
    dataset: &Dataset,
    population: &Population,
    read: &[NormalizedName],
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, InternalError> {
    let keys = key_columns(dataset, read, catalog)?;
    Ok(scan(
        &dataset.name,
        &keys,
        &Origin::Model(population.location.clone()),
    ))
}

/// The rule of `population` over the columns of its `Scan`, which names each field after its
/// normalized name; `required_when` reads those names as stored (E0H-1.3).
pub(super) fn population_rule(population: &Population) -> PopulationRule {
    let membership = match &population.mode {
        PopulationMode::Validity {
            valid_from,
            valid_to,
            partial_period,
        } => Membership::Validity {
            valid_from: ColumnName::new(valid_from.as_str()),
            valid_to: ColumnName::new(valid_to.as_str()),
            partial_period: *partial_period,
        },
        PopulationMode::Periodic { period, .. } => Membership::Periodic {
            period: ColumnName::new(period.as_str()),
        },
    };
    PopulationRule {
        entity: columns(&population.entity),
        membership,
        required_when: population.required_when.clone(),
    }
}

/// The `Scan` columns of `names`.
pub(super) fn columns(names: &[NormalizedName]) -> Vec<ColumnName> {
    names
        .iter()
        .map(|name| ColumnName::new(name.as_str()))
        .collect()
}
