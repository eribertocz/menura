//! The `producer` of a compiled plan (PLN-1, VS-5, E6-5.5).

/// The normative Apache Ossie release (section 4.1).
pub(crate) const OSSIE_NORMATIVE: &str = "0.1.1";

/// The Apache Ossie development commit the compiler tracks, as
/// `conformance/ossie/upstream.json` records it (OSS-5.1).
pub(crate) const OSSIE_TRACKED_COMMIT: &str = "aa500d37894c3d62c96cdce4192cbecea4d7f06a";

#[cfg(test)]
mod tests {
    use super::{OSSIE_NORMATIVE, OSSIE_TRACKED_COMMIT};

    const UPSTREAM: &str = include_str!("../../../conformance/ossie/upstream.json");

    fn upstream(member: &str) -> String {
        let document: serde_json::Value = serde_json::from_str(UPSTREAM).unwrap();
        document[member].as_str().unwrap().to_owned()
    }

    #[test]
    fn vs_5_the_tracked_commit_is_the_one_upstream_json_records() {
        assert_eq!(OSSIE_TRACKED_COMMIT, upstream("tracked_commit"));
    }

    #[test]
    fn vs_5_the_normative_release_is_the_one_upstream_json_records() {
        let release = upstream("recorded_release");
        let version = release.strip_prefix("osi-").unwrap();
        let version = version.split('-').next().unwrap();
        assert_eq!(OSSIE_NORMATIVE, version);
    }
}
