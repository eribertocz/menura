//! `wrapper-parity <conformance-directory> --cli <menura> --mcp <menura-mcp> --manifest <file>
//! --work <directory>`: runs every conformance case through the CLI and the MCP server and
//! compares their documents with those of `core`, byte for byte, then writes the calls and
//! their expected documents to `--manifest` for the Python package's check (ARC-6.9, A4).
//! Every difference is printed with its case, its wrapper and its first differing line; any
//! difference exits `1`.

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    reason = "A2: like the command-line wrapper, the parity check reads its arguments and runs \
              the wrapper binaries"
)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use menura_conformance_runner::wrappers::{
    CliForm, Expected, McpForm, Mismatch, ParityError, WrapperCall, calls_of_cases,
    expected_result, usage_probes, write_manifest,
};

const USAGE: &str = "usage: wrapper-parity <conformance-directory> --cli <menura> \
                     --mcp <menura-mcp> --manifest <file> --work <directory>";

/// The command line, once parsed.
struct Arguments {
    conformance: PathBuf,
    cli: PathBuf,
    mcp: PathBuf,
    manifest: PathBuf,
    work: PathBuf,
}

fn main() -> ExitCode {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let Some(arguments) = parse(&arguments) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match check(&arguments) {
        Ok(mismatches) if mismatches.is_empty() => ExitCode::SUCCESS,
        Ok(mismatches) => {
            for mismatch in &mismatches {
                println!("{mismatch}");
            }
            println!("{} differences", mismatches.len());
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

/// The arguments: a directory, then each flag once with its value, in any order.
fn parse(arguments: &[OsString]) -> Option<Arguments> {
    let (conformance, flags) = arguments.split_first()?;
    let value = |name: &str| -> Option<PathBuf> {
        let mut found = flags
            .chunks(2)
            .filter(|pair| pair.first().is_some_and(|flag| flag == name));
        let pair = found.next()?;
        found
            .next()
            .is_none()
            .then(|| pair.get(1).map(PathBuf::from))?
    };
    (flags.len() == 8).then_some(())?;
    Some(Arguments {
        conformance: PathBuf::from(conformance),
        cli: value("--cli")?,
        mcp: value("--mcp")?,
        manifest: value("--manifest")?,
        work: value("--work")?,
    })
}

/// Runs every call through the CLI and MCP, writes the manifest, and returns the differences.
fn check(arguments: &Arguments) -> Result<Vec<Mismatch>, String> {
    let directory = &arguments.conformance;
    let mut calls = calls_of_cases(directory).map_err(|error| error.to_string())?;
    calls.extend(usage_probes(directory).map_err(|error| error.to_string())?);
    let expected = calls
        .into_iter()
        .map(|call| expected_result(&call).map(|expected| (call, expected)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mismatches = run_wrappers(arguments, &expected).map_err(|error| error.to_string())?;
    write_manifest(&arguments.manifest, &expected).map_err(|error| error.to_string())?;
    let skipped = expected
        .iter()
        .filter(|(call, _)| !McpForm::accepts(call))
        .count();
    println!(
        "{} calls through the CLI, {} through MCP ({skipped} not UTF-8)",
        expected.len(),
        expected.len() - skipped
    );
    Ok(mismatches)
}

/// The differences of every call through the CLI and, when its inputs are UTF-8, MCP.
fn run_wrappers(
    arguments: &Arguments,
    calls: &[(WrapperCall, Expected)],
) -> Result<Vec<Mismatch>, ParityError> {
    std::fs::create_dir_all(&arguments.work).map_err(|error| ParityError::Io {
        path: arguments.work.clone(),
        error,
    })?;
    let cli = CliForm::new(arguments.cli.clone(), arguments.work.clone());
    let mut mcp = McpForm::start(Path::new(&arguments.mcp))?;
    let mut mismatches = Vec::new();
    for (call, expected) in calls {
        mismatches.extend(cli.check(call, expected)?);
        if McpForm::accepts(call) {
            mismatches.extend(mcp.check(call, expected)?);
        }
    }
    Ok(mismatches)
}
