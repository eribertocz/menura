//! Counts instructions with Valgrind Callgrind, run as an external tool (R7).

use std::path::Path;
use std::process::Command;

use crate::error::BenchmarkError;

/// The instructions of one compilation of `scenario`: the count of a run with two
/// compilations minus the count of a run with one, so that loading and the first-use costs
/// are not counted. `executable` is the benchmark binary; Callgrind writes its output files
/// in `directory`.
///
/// # Errors
///
/// [`BenchmarkError::Io`] when Valgrind cannot be run, fails, or writes no total.
pub fn count_compilation(
    executable: &Path,
    scenario: &str,
    directory: &Path,
) -> Result<u64, BenchmarkError> {
    let one = count_run(executable, scenario, 1, directory)?;
    let two = count_run(executable, scenario, 2, directory)?;
    Ok(two.saturating_sub(one))
}

fn count_run(
    executable: &Path,
    scenario: &str,
    compilations: usize,
    directory: &Path,
) -> Result<u64, BenchmarkError> {
    let output_file = directory.join(format!("{scenario}-{compilations}.callgrind"));
    let status = Command::new("valgrind")
        .arg("--tool=callgrind")
        .arg(format!("--callgrind-out-file={}", output_file.display()))
        .arg("--quiet")
        .arg(executable)
        .args(["scenario", scenario, &compilations.to_string()])
        .status()
        .map_err(|error| BenchmarkError::Io(format!("cannot run valgrind: {error}")))?;
    if !status.success() {
        return Err(BenchmarkError::Io(format!(
            "valgrind failed on scenario `{scenario}`: {status}"
        )));
    }
    let text = std::fs::read_to_string(&output_file).map_err(|error| {
        BenchmarkError::Io(format!("cannot read {}: {error}", output_file.display()))
    })?;
    total_instructions(&text)
        .ok_or_else(|| BenchmarkError::Io(format!("no total in {}", output_file.display())))
}

/// The total instruction count of a Callgrind output file: its `summary:` line, or else its
/// `totals:` line.
///
/// # Example
///
/// ```
/// use menura_benchmarks::total_instructions;
///
/// assert_eq!(total_instructions("events: Ir\nsummary: 1234\n"), Some(1234));
/// ```
#[must_use]
pub fn total_instructions(text: &str) -> Option<u64> {
    ["summary:", "totals:"].iter().find_map(|prefix| {
        text.lines()
            .find_map(|line| line.strip_prefix(prefix))
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse().ok())
    })
}
