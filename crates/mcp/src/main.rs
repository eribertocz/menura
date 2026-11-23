//! `menura-mcp`: the MCP server over standard input and output (ARC-6.1, ARC-6.6). It reads
//! one JSON-RPC message per line and writes each answer as one line; it writes nothing else to
//! standard output and opens no file or network connection.

#![forbid(unsafe_code)]

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match serve() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("menura-mcp: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Answers every message of standard input until it ends.
fn serve() -> io::Result<()> {
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(answer) = menura_mcp::answer(&line) {
            writeln!(output, "{answer}")?;
            output.flush()?;
        }
    }
    Ok(())
}
