//! Runs the query cases of the 0.3.0 slice (CS-2.8): every case compiles through `core`, every
//! plan is executed on `DuckDB` (E6-5.4), and only the cases whose query needs a capability
//! outside the slice are pending (VS-4).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use menura_conformance_runner::{CaseStatus, run};

const AREAS: [&str; 7] = ["cs", "e1", "e2", "e4", "e6", "qry", "sec"];

/// The cases whose query needs a capability outside the 0.3.0 slice.
const OUTSIDE_THE_SLICE: [&str; 2] = ["cs-not-yet-supported-001", "e2-policy-join-001"];

fn conformance_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

#[test]
fn cs_2_8_every_query_case_of_the_slice_passes() {
    let mut problems = Vec::new();
    let mut pending = BTreeSet::new();
    for area in AREAS {
        for case in run(&conformance_directory(), Some(area)).unwrap().cases {
            match case.status {
                CaseStatus::Passed => {}
                CaseStatus::Pending(reason) if reason.contains("E_NOT_YET_SUPPORTED") => {
                    pending.insert(case.id.as_str().to_owned());
                }
                status => problems.push(format!("{}: {status:?}", case.id)),
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(
        pending,
        BTreeSet::from(OUTSIDE_THE_SLICE.map(str::to_owned))
    );
}
