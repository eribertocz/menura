//! The argument and option validation of every command (ARC-5.5 to ARC-5.10), shared by the
//! wrappers: each wrapper reads its own syntax, names arguments with the neutral names of
//! [`Argument`], and renders a [`UsageError`] with its own spelling (`--model` for the CLI,
//! `model` for an MCP member).

mod argument;
mod check;
mod command;
mod error;

pub use argument::Argument;
pub use check::{check_audit_record, parse_compile_options, parse_dialect, parse_load_options};
pub use command::CommandName;
pub use error::{UsageError, UsageReason};
