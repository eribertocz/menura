//! Wrapper parity (ARC-6.9, A4): every conformance case run through the CLI, the MCP server and
//! the Python package gives the documents `core` gives, byte for byte.
//!
//! [`calls_of_cases`] turns the cases of a conformance directory into wrapper calls, and
//! [`expected_result`] computes, through `core`, what each call must give. [`CliForm`] and
//! [`McpForm`] run a call through the `menura` and `menura-mcp` binaries and report every
//! [`Mismatch`]; [`write_manifest`] writes the calls and their expected documents for the
//! Python script `scripts/ci/check_wrapper_parity.py`, which checks the package.

mod call;
mod case_calls;
mod cli_form;
mod difference;
mod error;
mod expected;
mod manifest;
mod mcp_form;

pub use call::{CallValue, Document, WrapperCall};
pub use case_calls::{calls_of_cases, usage_probes};
pub use cli_form::CliForm;
pub use difference::{LineDifference, Mismatch, first_difference};
pub use error::ParityError;
pub use expected::{Expected, ExpectedOutput, expected_result};
pub use manifest::write_manifest;
pub use mcp_form::McpForm;
