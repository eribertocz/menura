//! A2, A4 and ARC-6.1: `menura-mcp` is a wrapper that depends only on `core` among the
//! workspace crates, and uses no I/O but standard input and output.

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    reason = "A4: the test reads the crate's manifest and sources from disk"
)]

use std::fs;
use std::path::Path;

fn crate_file(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

/// The `[dependencies]` and `[dev-dependencies]` lines of the manifest that name a package.
fn dependency_names() -> Vec<String> {
    let manifest = crate_file("Cargo.toml");
    let mut names = Vec::new();
    let mut in_dependencies = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_dependencies = line.ends_with("dependencies]");
            continue;
        }
        if in_dependencies && let Some((name, _)) = line.split_once('=') {
            names.push(name.trim().trim_end_matches(".workspace").to_owned());
        }
    }
    names
}

#[test]
fn a4_the_server_depends_only_on_core_among_the_workspace_crates() {
    let workspace: Vec<String> = dependency_names()
        .into_iter()
        .filter(|name| name.starts_with("menura-"))
        .collect();
    assert_eq!(workspace, ["menura-core"]);
}

#[test]
fn a4_the_server_adds_no_external_dependency_but_serde_json() {
    let external: Vec<String> = dependency_names()
        .into_iter()
        .filter(|name| !name.starts_with("menura-"))
        .collect();
    assert_eq!(external, ["serde_json"]);
}

#[test]
fn a2_arc_6_1_the_sources_use_no_io_but_standard_input_and_output() {
    let sources = fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("src")).unwrap();
    for entry in sources {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for forbidden in [
            "std::fs",
            "std::net",
            "std::env",
            "std::process::Command",
            "std::time",
        ] {
            assert!(
                !text.contains(forbidden),
                "{} uses {forbidden}",
                path.display()
            );
        }
        for allowed in ["io::stdin", "io::stdout"] {
            if text.contains(allowed) {
                assert!(
                    path.ends_with("main.rs"),
                    "{} uses {allowed}",
                    path.display()
                );
            }
        }
    }
}
