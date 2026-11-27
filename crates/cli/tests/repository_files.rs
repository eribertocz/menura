//! PLT-1.6: every text file of the repository is checked out with line feed endings, so the
//! golden files compare byte for byte on every platform. The `platforms` job runs this test on
//! the macOS and Windows checkouts.

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "PLT-1.6: the test lists the repository's files with git and reads them"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The line of `.gitattributes` that stores and checks out every text file with line feeds.
const LINE_FEED_ATTRIBUTE: &str = "* text=auto eol=lf";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The binary file extensions of `.gitattributes`: the `<pattern> binary` lines.
fn binary_extensions(attributes: &str) -> Vec<String> {
    attributes
        .lines()
        .filter_map(|line| line.strip_suffix(" binary"))
        .filter_map(|pattern| pattern.strip_prefix("*."))
        .map(str::to_owned)
        .collect()
}

/// Every file git tracks, relative to the workspace root.
fn tracked_files() -> Vec<String> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(workspace())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout)
        .unwrap()
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn plt_1_6_gitattributes_checks_out_every_text_file_with_line_feeds() {
    let attributes = fs::read_to_string(workspace().join(".gitattributes")).unwrap();
    assert!(
        attributes.lines().any(|line| line == LINE_FEED_ATTRIBUTE),
        "{attributes}"
    );
}

#[test]
fn plt_1_6_no_tracked_text_file_holds_a_carriage_return() {
    let attributes = fs::read_to_string(workspace().join(".gitattributes")).unwrap();
    let binary = binary_extensions(&attributes);
    let files = tracked_files();
    assert!(files.len() > 100, "{} tracked files", files.len());
    let mut with_carriage_returns = Vec::new();
    for name in files {
        let path = workspace().join(&name);
        let is_binary = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| binary.iter().any(|known| known == extension));
        let bytes = fs::read(&path).unwrap();
        if !is_binary && bytes.contains(&b'\r') {
            with_carriage_returns.push(name);
        }
    }
    assert!(
        with_carriage_returns.is_empty(),
        "{with_carriage_returns:?}"
    );
}
