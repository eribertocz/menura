//! Workspace dependency graph test: dependencies between crates point in one direction (S6).

#![expect(
    clippy::disallowed_methods,
    reason = "S6: the graph test reads the workspace manifests from disk; A2 allows I/O in the cli crate only"
)]

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Prefix shared by every package of this workspace.
const PACKAGE_PREFIX: &str = "semantic-compiler-";

/// Layers of `spec/05-engineering-standards.md` §5.2, from the bottom up.
/// A crate may depend only on crates of a strictly lower layer.
const LAYERS: &[&[&str]] = &[
    &["contracts"],
    &["model"],
    &["expressions"],
    &["representations"],
    &["security"],
    &[
        "e0-load",
        "e1-parse",
        "e2-bind",
        "e3-paths",
        "e4-plan",
        "e5-optimize",
    ],
    &["generator-duckdb"],
    &["core"],
    &["cli"],
];

/// The exact set of dependency edges between workspace crates, as `(dependent, dependency)`.
const EXPECTED_EDGES: &[(&str, &str)] = &[
    ("model", "contracts"),
    ("expressions", "model"),
    ("representations", "model"),
    ("security", "model"),
    ("security", "representations"),
    ("e0-load", "security"),
    ("e1-parse", "security"),
    ("e2-bind", "security"),
    ("e3-paths", "security"),
    ("e4-plan", "security"),
    ("e5-optimize", "security"),
    ("generator-duckdb", "e5-optimize"),
    ("core", "e0-load"),
    ("core", "e1-parse"),
    ("core", "e2-bind"),
    ("core", "e3-paths"),
    ("core", "e4-plan"),
    ("core", "e5-optimize"),
    ("core", "generator-duckdb"),
    ("cli", "core"),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn crate_names() -> Vec<&'static str> {
    LAYERS
        .iter()
        .flat_map(|layer| layer.iter().copied())
        .collect()
}

fn layer_of(crate_name: &str) -> Option<usize> {
    LAYERS.iter().position(|layer| layer.contains(&crate_name))
}

/// Returns whether `dependent` may depend on `dependency`: both are in a layer and the
/// dependency's layer is strictly lower.
fn is_allowed_edge(dependent: &str, dependency: &str) -> bool {
    match (layer_of(dependent), layer_of(dependency)) {
        (Some(dependent_layer), Some(dependency_layer)) => dependency_layer < dependent_layer,
        _ => false,
    }
}

/// Returns the workspace crates named as dependencies (of any kind) in a manifest.
fn workspace_dependencies(manifest: &str) -> BTreeSet<String> {
    let mut dependencies = BTreeSet::new();
    let mut in_dependency_table = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_dependency_table = line.ends_with("dependencies]");
            continue;
        }
        if !in_dependency_table {
            continue;
        }
        let key = line.split(['=', '.', ' ']).next().unwrap_or_default();
        if let Some(name) = key.strip_prefix(PACKAGE_PREFIX) {
            dependencies.insert(name.to_owned());
        }
    }
    dependencies
}

fn actual_edges() -> io::Result<BTreeSet<(String, String)>> {
    let root = workspace_root();
    let mut edges = BTreeSet::new();
    for name in crate_names() {
        let path = root.join("crates").join(name).join("Cargo.toml");
        let manifest = fs::read_to_string(&path)?;
        for dependency in workspace_dependencies(&manifest) {
            edges.insert((name.to_owned(), dependency));
        }
    }
    Ok(edges)
}

#[test]
fn s6_every_workspace_member_is_in_a_layer() {
    let manifest = fs::read_to_string(workspace_root().join("Cargo.toml")).unwrap();
    let members: BTreeSet<&str> = manifest
        .lines()
        .filter_map(|line| line.trim().strip_prefix("\"crates/"))
        .map(|rest| rest.trim_end_matches("\","))
        .collect();
    let layered: BTreeSet<&str> = crate_names().into_iter().collect();
    assert_eq!(members, layered);
}

#[test]
fn s6_dependencies_point_to_lower_layers_only() {
    for (dependent, dependency) in actual_edges().unwrap() {
        assert!(
            is_allowed_edge(&dependent, &dependency),
            "`{dependent}` depends on `{dependency}`, which is not in a lower layer"
        );
    }
}

#[test]
fn s6_dependency_edges_match_the_specification() {
    let expected: BTreeSet<(String, String)> = EXPECTED_EDGES
        .iter()
        .map(|(dependent, dependency)| ((*dependent).to_owned(), (*dependency).to_owned()))
        .collect();
    assert_eq!(actual_edges().unwrap(), expected);
}

/// The shared crates of §5.2, in their required order from the bottom up.
const SHARED_CRATES: &[&str] = &["model", "expressions", "representations", "security"];

#[test]
fn s6_shared_crates_are_layered_in_order() {
    let order: Vec<&str> = crate_names()
        .into_iter()
        .filter(|name| SHARED_CRATES.contains(name))
        .collect();
    assert_eq!(order, SHARED_CRATES);
}

#[test]
fn s10_expressions_may_not_depend_on_a_stage() {
    assert!(!is_allowed_edge("expressions", "e0-load"));
}

#[test]
fn s9_representations_may_not_depend_on_security() {
    assert!(!is_allowed_edge("representations", "security"));
}

#[test]
fn s9_representations_may_not_depend_on_a_stage() {
    assert!(!is_allowed_edge("representations", "e0-load"));
}

#[test]
fn s9_s10_specified_shared_crate_edges_are_allowed() {
    assert!(is_allowed_edge("expressions", "model"));
    assert!(is_allowed_edge("representations", "model"));
    assert!(is_allowed_edge("security", "representations"));
}
