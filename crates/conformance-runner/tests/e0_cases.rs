//! Runs the cases of `conformance/cases/e0/` through `core` (CS-2.8): every case passes, because
//! no compilation returns `E_NOT_YET_SUPPORTED` from 1.0.0 (VS-3).

use std::path::{Path, PathBuf};

use menura_conformance_runner::{CaseStatus, run};

fn conformance_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

#[test]
fn cs_2_8_every_e0_case_passes() {
    let report = run(&conformance_directory(), Some("e0")).unwrap();
    let problems: Vec<String> = report
        .cases
        .iter()
        .filter(|case| case.status != CaseStatus::Passed)
        .map(|case| format!("{}: {:?}", case.id, case.status))
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
