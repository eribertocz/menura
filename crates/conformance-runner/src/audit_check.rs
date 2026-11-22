//! The `audit` expectation (CS-2.13): the audit record's `producer` is the build's, and the
//! rest of the record equals its golden file.

use std::path::Path;

use menura_contracts::types::audit_record::AuditRecordDocument;
use menura_contracts::types::common::Producer;

use crate::canonical_json::JsonValue;
use crate::golden_json::golden_mismatch;

/// The member of the audit record that is checked against the build and left out of the golden
/// file (CS-2.13).
const PRODUCER: &str = "producer";

/// Why the audit record does not meet the `audit` expectation `file_name`, or `None`: its
/// `producer` must equal `build_producer`, the producer of the build under test, and the record
/// without `producer` must equal the golden file (CS-2.13, CS-2.10).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "compared once core returns the audit record; until then the audit part is pending (CS-2.8)"
    )
)]
pub(crate) fn audit_mismatch(
    record: &AuditRecordDocument,
    build_producer: &Producer,
    case_folder: &Path,
    file_name: &str,
) -> Option<String> {
    if record.producer != *build_producer {
        return Some(format!(
            "the audit record's producer is not the producer of the build under test: {:?}",
            record.producer
        ));
    }
    match JsonValue::of(record) {
        Ok(record) => golden_mismatch(case_folder, file_name, &record.without(&[PRODUCER])),
        Err(reason) => Some(format!("the audit record cannot be read as JSON: {reason}")),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const RECORD: &str = include_str!("../../../spec/contracts/examples/audit-record.example.json");

    fn record() -> AuditRecordDocument {
        serde_json::from_str(RECORD).unwrap()
    }

    fn case_folder() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases/cs")
    }

    #[test]
    fn cs_2_13_the_record_without_producer_matches_its_golden_file() {
        let record = record();
        let reason = audit_mismatch(
            &record,
            &record.producer,
            &case_folder(),
            "cs-unit-audit-001.audit.json",
        );
        assert_eq!(reason, None);
    }

    #[test]
    fn cs_2_13_another_producer_fails_before_the_golden_file_is_read() {
        let record = record();
        let mut other = record.producer.clone();
        other.tzdata_version = "1999z".to_owned();
        let reason = audit_mismatch(&record, &other, &case_folder(), "no-such-file.json").unwrap();
        assert!(reason.contains("producer"), "{reason}");
    }

    #[test]
    fn cs_2_13_a_different_record_fails() {
        let mut record = record();
        record.row_policies.push("another_policy".to_owned());
        let producer = record.producer.clone();
        let reason = audit_mismatch(
            &record,
            &producer,
            &case_folder(),
            "cs-unit-audit-001.audit.json",
        )
        .unwrap();
        assert!(reason.contains("cs-unit-audit-001.audit.json"), "{reason}");
    }
}
