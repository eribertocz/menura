//! Runs the query cases of the 0.3.0 slice (CS-2.8): every case compiles through `core`, every
//! plan is executed on `DuckDB` (E6-5.4), and every case passes, including those that expect
//! `E_NOT_YET_SUPPORTED` (VS-4).

use std::path::{Path, PathBuf};

use menura_conformance_runner::{CaseStatus, run};

const AREAS: [&str; 7] = ["cs", "e1", "e2", "e4", "e6", "qry", "sec"];

fn conformance_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

#[test]
fn cs_2_8_every_query_case_of_the_slice_passes() {
    let mut problems = Vec::new();
    for area in AREAS {
        for case in run(&conformance_directory(), Some(area)).unwrap().cases {
            if case.status != CaseStatus::Passed {
                problems.push(format!("{}: {:?}", case.id, case.status));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
