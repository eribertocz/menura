//! The `producer` of a compiled plan and of an audit record (PLN-1, PLN-6, E6-5.5).

use menura_contracts::ContractVersion;
use menura_contracts::types::common::Producer;
use menura_e1_parse::time_zone_database_version;

use crate::compile::InternalError;

/// The compiler version: the workspace version.
const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The normative Apache Ossie release (section 4.1).
pub(crate) const OSSIE_NORMATIVE: &str = "0.1.1";

/// The Apache Ossie development commit the compiler tracks, as
/// `conformance/ossie/upstream.json` records it (OSS-5.1).
pub(crate) const OSSIE_TRACKED_COMMIT: &str = "aa500d37894c3d62c96cdce4192cbecea4d7f06a";

/// The producer of every compiled plan: the compiler version, the Apache Ossie release and
/// commit, and the version of the time-zone database E1 uses (E6-5.5).
///
/// # Errors
///
/// An [`InternalError`] when the workspace version is not `MAJOR.MINOR.PATCH`.
pub(crate) fn producer() -> Result<Producer, InternalError> {
    let compiler_version = ContractVersion::parse(COMPILER_VERSION).ok_or_else(|| {
        InternalError::new(
            "core",
            format!("the compiler version `{COMPILER_VERSION}` is not MAJOR.MINOR.PATCH"),
        )
    })?;
    Ok(Producer {
        compiler_version,
        ossie_normative: OSSIE_NORMATIVE.to_owned(),
        ossie_tracked_commit: OSSIE_TRACKED_COMMIT.to_owned(),
        tzdata_version: time_zone_database_version().to_owned(),
    })
}

/// The producer of this build, as every compiled plan and audit record writes it (PLN-1,
/// E6-5.5); `None` only when the workspace version is not `MAJOR.MINOR.PATCH`.
///
/// # Example
///
/// ```
/// let producer = menura_core::build_producer().unwrap();
/// assert_eq!(producer.ossie_normative, "0.1.1");
/// ```
#[must_use]
pub fn build_producer() -> Option<Producer> {
    producer().ok()
}

#[cfg(test)]
mod tests {
    use super::{OSSIE_NORMATIVE, OSSIE_TRACKED_COMMIT};

    const UPSTREAM: &str = include_str!("../../../conformance/ossie/upstream.json");

    fn upstream(member: &str) -> String {
        let document: serde_json::Value = serde_json::from_str(UPSTREAM).unwrap();
        document[member].as_str().unwrap().to_owned()
    }

    #[test]
    fn pln_1_the_tracked_commit_is_the_one_upstream_json_records() {
        assert_eq!(OSSIE_TRACKED_COMMIT, upstream("tracked_commit"));
    }

    #[test]
    fn pln_1_the_normative_release_is_the_one_upstream_json_records() {
        let release = upstream("recorded_release");
        let version = release.strip_prefix("osi-").unwrap();
        let version = version.split('-').next().unwrap();
        assert_eq!(OSSIE_NORMATIVE, version);
    }
}
