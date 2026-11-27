//! `menura-mcp`: the MCP server over standard input and output (ARC-6.1, ARC-6.6). It reads
//! one JSON-RPC message per line and writes each answer as one line; it writes nothing else to
//! standard output and opens no file or network connection. Its lines end with a line feed
//! only, on every platform (PLT-1.4).

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

/// Answers every message of standard input until it ends. Each answer is written as bytes and
/// ends with one line feed, never a carriage return, on every platform (PLT-1.4).
fn serve() -> io::Result<()> {
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(answer) = menura_mcp::answer(&line) {
            output.write_all(answer.as_bytes())?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
    Ok(())
}
