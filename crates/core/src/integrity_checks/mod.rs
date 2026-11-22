//! The public function `integrity_checks` (architecture section 3.5): the checks E0 recorded
//! (E0H-4.3), each generated from its logical plan by the dialect's generator (CHK-4.2).

mod key_columns;
mod plan_nodes;
mod relationship_plans;
mod result_names;

use menura_contracts::ContractKind;
use menura_contracts::types::common::Dialect;
use menura_contracts::types::compiled_plan::{CompileErrorContract, CompileErrorDocument};
use menura_contracts::types::descriptions::{
    CheckKind as ContractCheckKind, CheckParameter, IntegrityCheck, IntegrityChecksContract,
    IntegrityChecksDocument,
};
use menura_generator_duckdb::generate_check_sql;
use menura_model::internal_model::{CheckKind, NormalizedName, RecordedCheck, Relationship};
use menura_representations::logical_plan::PlanNode;

use crate::compile::InternalError;
use crate::handle::Handle;
use crate::stop::Stop;
use key_columns::STAGE;
use relationship_plans::{key_uniqueness_plan, orphan_keys_plan};

/// Generates the integrity checks of the model of `handle` in `dialect` (E0H-4, CHK-2).
///
/// The document lists the `key_uniqueness` and `orphan_keys` checks of every usable
/// relationship, sorted by `check_id` (E0H-4.3). Each check's query is generated from its
/// logical plan by the same generator as `compile` (CHK-4.2); it returns one row per
/// violation (CHK-2.1, CHK-2.5) and applies no row policy (CHK-2.2). The other kinds of check
/// are not generated yet.
///
/// # Errors
///
/// The compile error document (PLN-5) with `E_INTERNAL`, without location, for a defect of the
/// compiler (PLN-5.1a), with the handle's warnings.
///
/// # Example
///
/// ```
/// use menura_core::{Dialect, LoadOptions, integrity_checks, load};
/// # let model = include_bytes!("../../tests/fixtures/shop/model.yaml");
/// # let catalog_schema = include_bytes!("../../tests/fixtures/shop/catalog-schema.json");
///
/// let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
/// let document = integrity_checks(&handle, Dialect::Duckdb).unwrap();
/// assert_eq!(document.model_hash, *handle.model().hash());
/// ```
pub fn integrity_checks(
    handle: &Handle,
    dialect: Dialect,
) -> Result<IntegrityChecksDocument, CompileErrorDocument> {
    generated_checks(handle, dialect).map_err(|error| CompileErrorDocument {
        contract: CompileErrorContract::CompileError,
        contract_version: ContractKind::CompileError.latest_version(),
        errors: vec![Stop::from(error).into_diagnostic()],
        warnings: handle.warnings().cloned().collect(),
    })
}

fn generated_checks(
    handle: &Handle,
    dialect: Dialect,
) -> Result<IntegrityChecksDocument, InternalError> {
    let checks = handle
        .model()
        .parts()
        .checks
        .iter()
        .filter(|recorded| generated_kind(recorded.kind))
        .map(|recorded| generated_check(handle, dialect, recorded))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IntegrityChecksDocument {
        contract: IntegrityChecksContract::IntegrityChecks,
        contract_version: ContractKind::IntegrityChecks.latest_version(),
        model_hash: handle.model().hash().clone(),
        dialect,
        checks,
    })
}

/// Whether checks of `kind` are generated in this version.
fn generated_kind(kind: CheckKind) -> bool {
    matches!(kind, CheckKind::KeyUniqueness | CheckKind::OrphanKeys)
}

/// The check `recorded` with its query in `dialect`.
fn generated_check(
    handle: &Handle,
    dialect: Dialect,
    recorded: &RecordedCheck,
) -> Result<IntegrityCheck, InternalError> {
    let model = handle.model();
    let catalog = handle.catalog_schema();
    let relationship = model
        .relationship(&NormalizedName::new(&recorded.target))
        .ok_or_else(|| {
            InternalError::new(
                STAGE,
                format!("the check {} names no relationship", recorded.check_id),
            )
        })?;
    let (kind, plan, read) = match recorded.kind {
        CheckKind::KeyUniqueness => (
            ContractCheckKind::KeyUniqueness,
            key_uniqueness_plan(relationship, model, catalog)?,
            vec![&relationship.to],
        ),
        CheckKind::OrphanKeys => (
            ContractCheckKind::OrphanKeys,
            orphan_keys_plan(relationship, model, catalog)?,
            vec![&relationship.from, &relationship.to],
        ),
        other => {
            return Err(InternalError::new(
                STAGE,
                format!("checks of kind {} are not generated yet", other.as_str()),
            ));
        }
    };
    let (sql, parameters) = generated_sql(handle, dialect, &plan)?;
    Ok(IntegrityCheck {
        check_id: recorded.check_id.clone(),
        kind,
        target: recorded.target.clone(),
        definition_hash: recorded.definition_hash.clone(),
        sql,
        parameters,
        sources: sources(handle, relationship, &read)?,
        build: None,
    })
}

/// The SQL of `plan` in `dialect` and its parameters, each a value the compiler fixed (CHK-7.1).
fn generated_sql(
    handle: &Handle,
    dialect: Dialect,
    plan: &PlanNode,
) -> Result<(String, Vec<CheckParameter>), InternalError> {
    match dialect {
        Dialect::Duckdb => {
            let generated = generate_check_sql(plan, handle.catalog_schema(), handle.model())
                .map_err(|error| InternalError::new("E6", format!("{error:?}")))?;
            let parameters = generated
                .parameters
                .into_iter()
                .map(CheckParameter::Fixed)
                .collect();
            Ok((generated.sql, parameters))
        }
    }
}

/// The `source` of each dataset of `read`, each once, sorted in CON-5 order (CHK-2.4).
fn sources(
    handle: &Handle,
    relationship: &Relationship,
    read: &[&NormalizedName],
) -> Result<Vec<String>, InternalError> {
    let mut sources = read
        .iter()
        .map(|name| {
            handle
                .model()
                .dataset(name)
                .map(|dataset| dataset.source.clone())
                .ok_or_else(|| {
                    InternalError::new(
                        STAGE,
                        format!(
                            "the relationship {} names the unknown dataset {}",
                            relationship.name.as_str(),
                            name.as_str()
                        ),
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    sources.sort();
    sources.dedup();
    Ok(sources)
}
