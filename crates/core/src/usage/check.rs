//! The checks of argument values: the options objects, the dialect and the audit record
//! (ARC-5.5, ARC-5.7, ARC-5.8).

use menura_contracts::types::common::Dialect;

use super::argument::Argument;
use super::error::{UsageError, UsageReason};
use crate::compile_options::CompileOptions;
use crate::load_options::LoadOptions;

/// The load options of `bytes`, a JSON object of load options (ARC-5.5).
///
/// # Errors
///
/// [`UsageReason::InvalidOptions`] about `load_options` when `bytes` is not one.
pub fn parse_load_options(bytes: &[u8]) -> Result<LoadOptions, UsageError> {
    LoadOptions::from_json(bytes).map_err(|error| invalid(Argument::LoadOptions, &error))
}

/// The compile options of `bytes`, a JSON object of compile options (ARC-5.7).
///
/// # Errors
///
/// [`UsageReason::InvalidOptions`] about `compile_options` when `bytes` is not one.
pub fn parse_compile_options(bytes: &[u8]) -> Result<CompileOptions, UsageError> {
    CompileOptions::from_json(bytes).map_err(|error| invalid(Argument::CompileOptions, &error))
}

/// The dialect named `name`: only `duckdb` (ARC-5.8).
///
/// # Errors
///
/// [`UsageReason::UnknownDialect`] about `dialect` for any other name.
///
/// ```
/// use menura_core::{Dialect, parse_dialect};
///
/// assert_eq!(parse_dialect("duckdb"), Ok(Dialect::Duckdb));
/// assert!(parse_dialect("postgres").is_err());
/// ```
pub fn parse_dialect(name: &str) -> Result<Dialect, UsageError> {
    match name {
        "duckdb" => Ok(Dialect::Duckdb),
        _ => Err(UsageError::new(
            Some(Argument::Dialect),
            UsageReason::UnknownDialect(name.to_owned()),
        )),
    }
}

/// Checks that the CLI's `compile` names an audit record exactly when `options` ask for one
/// (ARC-5.7). The MCP tool returns the record instead and takes no `audit_record` (ARC-6.7).
///
/// # Errors
///
/// [`UsageReason::AuditWithoutRecord`] or [`UsageReason::RecordWithoutAudit`].
pub fn check_audit_record(options: &CompileOptions, record_given: bool) -> Result<(), UsageError> {
    match (options.audit, record_given) {
        (true, false) => Err(UsageError::new(
            Some(Argument::CompileOptions),
            UsageReason::AuditWithoutRecord,
        )),
        (false, true) => Err(UsageError::new(
            Some(Argument::AuditRecord),
            UsageReason::RecordWithoutAudit,
        )),
        _ => Ok(()),
    }
}

fn invalid(argument: Argument, error: &impl std::fmt::Display) -> UsageError {
    UsageError::new(
        Some(argument),
        UsageReason::InvalidOptions(error.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_5_5_an_invalid_options_object_is_a_usage_error_about_its_argument() {
        let error = parse_load_options(br#"{"namespace": null}"#).unwrap_err();
        assert_eq!(error.argument, Some(Argument::LoadOptions));
        let error = parse_compile_options(b"[]").unwrap_err();
        assert_eq!(error.argument, Some(Argument::CompileOptions));
    }

    #[test]
    fn arc_5_7_audit_and_audit_record_go_together() {
        let audit = CompileOptions {
            audit: true,
            ..CompileOptions::default()
        };
        assert!(check_audit_record(&audit, true).is_ok());
        assert!(check_audit_record(&CompileOptions::default(), false).is_ok());
        assert_eq!(
            check_audit_record(&audit, false).map_err(|error| error.reason),
            Err(UsageReason::AuditWithoutRecord)
        );
        assert_eq!(
            check_audit_record(&CompileOptions::default(), true).map_err(|error| error.reason),
            Err(UsageReason::RecordWithoutAudit)
        );
    }
}
