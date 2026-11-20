//! Guard for the `jiff` exception of A2 (#229): `menura-e1-parse` may depend on `jiff` only
//! without its `std` feature, which is the only one that offers clock APIs such as
//! `Timestamp::now()`. The test fails if the resolved features of `jiff` include `std`.

#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the guard runs `cargo metadata`; A2 allows process access in the cli crate only"
)]

use std::process::Command;

use serde_json::Value;

/// The feature that gives `jiff` its clock (A2).
const CLOCK_FEATURE: &str = "std";

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).

/// The resolved dependency graph of the workspace, from `cargo metadata`.
#[cfg(test)]
fn resolved_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata prints JSON")
}

/// The resolved features of every `jiff` package of the graph.
#[cfg(test)]
fn jiff_features(metadata: &Value) -> Vec<Vec<String>> {
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .expect("the metadata has a resolved graph");
    nodes
        .iter()
        .filter(|node| node["id"].as_str().is_some_and(|id| id.contains("#jiff@")))
        .map(|node| {
            node["features"]
                .as_array()
                .expect("a node lists its features")
                .iter()
                .filter_map(|feature| feature.as_str().map(str::to_owned))
                .collect()
        })
        .collect()
}

#[test]
fn a2_jiff_is_resolved_without_its_std_feature() {
    let resolved = jiff_features(&resolved_metadata());
    assert!(!resolved.is_empty(), "jiff is in the graph (#229)");
    for features in resolved {
        assert!(
            !features.iter().any(|feature| feature == CLOCK_FEATURE),
            "jiff must be built without `{CLOCK_FEATURE}` (A2, #229); resolved features: {features:?}"
        );
    }
}
