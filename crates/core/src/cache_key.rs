//! The cache key of a compiled plan (PLN-3).

use menura_contracts::Sha256Hash;
use menura_contracts::hash_value;
use menura_contracts::types::catalog::{CatalogStateDocument, RelationData};
use menura_contracts::types::common::Dialect;
use menura_contracts::types::common::Parameter;
use menura_contracts::types::compiled_plan::{CacheKey, DataVersion, ResultColumn};
use serde_json::json;

/// The `*` period of a relation that is not partitioned (PLN-3.3).
const WHOLE_RELATION: &str = "*";

/// The parts of a compiled plan that `plan_hash` covers (PLN-3.1).
pub(crate) struct HashedPlan<'plan> {
    /// The target dialect.
    pub(crate) dialect: Dialect,
    /// The logical source name.
    pub(crate) data_source: &'plan str,
    /// The SQL statement.
    pub(crate) sql: &'plan str,
    /// The bound parameters.
    pub(crate) parameters: &'plan [Parameter],
    /// The output columns.
    pub(crate) result_schema: &'plan [ResultColumn],
}

/// The cache key of `plan` for the context of `security_fingerprint`, which reads the catalog
/// relations `sources` (PLN-3).
pub(crate) fn cache_key(
    plan: &HashedPlan<'_>,
    security_fingerprint: Sha256Hash,
    state: Option<&CatalogStateDocument>,
    sources: &[&str],
) -> CacheKey {
    let plan_hash = hash_value(&json!({
        "dialect": plan.dialect,
        "data_source": plan.data_source,
        "sql": plan.sql,
        "parameters": plan.parameters,
        "result_schema": plan.result_schema,
    }));
    let data_versions = state.and_then(|state| data_versions(state, sources));
    let key = hash_value(&json!({
        "plan_hash": plan_hash,
        "security_fingerprint": security_fingerprint,
        "data_versions": data_versions,
    }));
    CacheKey {
        key,
        plan_hash,
        security_fingerprint,
        data_versions,
    }
}

/// The versions `state` gives for the relations `sources`, sorted by source then period; `None`
/// when it gives none for one of them (PLN-3.3). Every period of a partitioned relation is
/// read, because 0.3.0 prunes no partition.
fn data_versions(state: &CatalogStateDocument, sources: &[&str]) -> Option<Vec<DataVersion>> {
    let mut versions = Vec::new();
    for source in sources {
        let relation = state
            .relations
            .iter()
            .find(|relation| relation.source == *source)?;
        match &relation.data {
            RelationData::Version(version) => versions.push(DataVersion {
                source: relation.source.clone(),
                period: WHOLE_RELATION.to_owned(),
                version: version.clone(),
            }),
            RelationData::Partitions(partitions) => {
                versions.extend(period_versions(source, &partitions.versions));
            }
            RelationData::AggregateBuild(build) => {
                versions.extend(period_versions(source, &build.built_from));
            }
        }
    }
    versions.sort_by(|left, right| {
        (left.source.as_str(), left.period.as_str())
            .cmp(&(right.source.as_str(), right.period.as_str()))
    });
    versions.dedup();
    Some(versions)
}

fn period_versions<'state>(
    source: &'state str,
    periods: &'state [menura_contracts::types::catalog::PeriodVersion],
) -> impl Iterator<Item = DataVersion> + 'state {
    periods.iter().map(move |period| DataVersion {
        source: source.to_owned(),
        period: period.period.clone(),
        version: period.version.clone(),
    })
}
