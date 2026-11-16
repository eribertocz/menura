//! Admission of incoming contract documents, with each contract's error codes
//! (CAT-9, QRY-7, SEC-4).
//!
//! Each check is a separate function so that the stage that admits a document runs them in the
//! order the specification gives for it (E1-2.1 for queries, E2-5 row 0a for security contexts).

use std::fmt;

use serde_json::{Map, Number, Value};

use crate::contract::ContractKind;
use crate::schema::validate_structure;
use crate::types::ContractDocument;
use crate::types::compiled_plan::InputDocument;

/// An incoming contract document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IncomingContract {
    /// `semantic_compiler.catalog_schema`.
    CatalogSchema,
    /// `semantic_compiler.catalog_state`.
    CatalogState,
    /// `semantic_compiler.query`.
    Query,
    /// `semantic_compiler.security_context`.
    SecurityContext,
}

/// A specification code raised while admitting a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdmissionCode {
    /// `ECAT-01`: the catalog schema does not satisfy its JSON Schema.
    Ecat01,
    /// `ECAT-02`: catalog schema `contract` or `contract_version` not supported.
    Ecat02,
    /// `ECAT-03`: the catalog schema violates a CAT-8 rule.
    Ecat03,
    /// `WCAT-01`: the catalog state does not satisfy its JSON Schema.
    Wcat01,
    /// `WCAT-02`: catalog state `contract` or `contract_version` not supported.
    Wcat02,
    /// `WCAT-03`: the catalog state violates a CAT-8 rule.
    Wcat03,
    /// `WCAT-04`: the catalog state's `schema_document_id` does not match the schema in use.
    Wcat04,
    /// `E_INVALID_QUERY`: the query is not JSON or fails its JSON Schema.
    InvalidQuery,
    /// `EQRY-01`: both or neither of `query` and `sql`.
    Eqry01,
    /// `EQRY-08`: query `contract` or `contract_version` not supported.
    Eqry08,
    /// `ESEC-01`: the security context fails its JSON Schema.
    Esec01,
    /// `ESEC-03`: security context `contract` or `contract_version` not supported.
    Esec03,
}

impl AdmissionCode {
    /// The code as the specification writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ecat01 => "ECAT-01",
            Self::Ecat02 => "ECAT-02",
            Self::Ecat03 => "ECAT-03",
            Self::Wcat01 => "WCAT-01",
            Self::Wcat02 => "WCAT-02",
            Self::Wcat03 => "WCAT-03",
            Self::Wcat04 => "WCAT-04",
            Self::InvalidQuery => "E_INVALID_QUERY",
            Self::Eqry01 => "EQRY-01",
            Self::Eqry08 => "EQRY-08",
            Self::Esec01 => "ESEC-01",
            Self::Esec03 => "ESEC-03",
        }
    }

    /// Whether the code is a warning: the document is ignored and compilation goes on (CAT-9).
    /// Every other code makes the operation fail.
    #[must_use]
    pub fn is_warning(self) -> bool {
        matches!(
            self,
            Self::Wcat01 | Self::Wcat02 | Self::Wcat03 | Self::Wcat04
        )
    }
}

impl fmt::Display for AdmissionCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A document was not admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionError {
    /// Specification code.
    pub code: AdmissionCode,
    /// The document that was not admitted.
    pub document: InputDocument,
    /// JSON Pointer (RFC 6901) of the offending value.
    pub pointer: String,
    /// Line and column (from 1) of a JSON syntax error.
    pub line_column: Option<(u64, u64)>,
    /// English description naming the offending item.
    pub message: String,
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} (at `{}`)",
            self.code, self.message, self.pointer
        )
    }
}

impl std::error::Error for AdmissionError {}

impl IncomingContract {
    /// The contract this document follows.
    #[must_use]
    pub fn kind(self) -> ContractKind {
        match self {
            Self::CatalogSchema => ContractKind::CatalogSchema,
            Self::CatalogState => ContractKind::CatalogState,
            Self::Query => ContractKind::Query,
            Self::SecurityContext => ContractKind::SecurityContext,
        }
    }

    /// The input document, as diagnostics locate it.
    #[must_use]
    pub fn document(self) -> InputDocument {
        match self {
            Self::CatalogSchema => InputDocument::CatalogSchema,
            Self::CatalogState => InputDocument::CatalogState,
            Self::Query => InputDocument::Query,
            Self::SecurityContext => InputDocument::SecurityContext,
        }
    }

    /// Code for a document that is not JSON or fails its JSON Schema.
    #[must_use]
    pub fn invalid_code(self) -> AdmissionCode {
        match self {
            Self::CatalogSchema => AdmissionCode::Ecat01,
            Self::CatalogState => AdmissionCode::Wcat01,
            Self::Query => AdmissionCode::InvalidQuery,
            Self::SecurityContext => AdmissionCode::Esec01,
        }
    }

    /// Code for an unsupported `contract` or `contract_version`.
    #[must_use]
    pub fn unsupported_code(self) -> AdmissionCode {
        match self {
            Self::CatalogSchema => AdmissionCode::Ecat02,
            Self::CatalogState => AdmissionCode::Wcat02,
            Self::Query => AdmissionCode::Eqry08,
            Self::SecurityContext => AdmissionCode::Esec03,
        }
    }

    /// Parses the document text as JSON.
    ///
    /// # Errors
    ///
    /// [`Self::invalid_code`] when the text is not JSON, with its line and column.
    pub fn parse_json(self, text: &str) -> Result<Value, AdmissionError> {
        serde_json::from_str(text).map_err(|error| AdmissionError {
            code: self.invalid_code(),
            document: self.document(),
            pointer: String::new(),
            line_column: Some((error.line() as u64, error.column() as u64)),
            message: format!("the document is not valid JSON: {error}"),
        })
    }

    /// Checks that `contract` names this contract and `contract_version` is accepted (CAT-3.2).
    ///
    /// # Errors
    ///
    /// [`Self::unsupported_code`], located at `/contract` or `/contract_version`.
    pub fn check_contract(self, document: &Value) -> Result<(), AdmissionError> {
        let kind = self.kind();
        if kind.supports(document) {
            return Ok(());
        }
        let names_contract =
            document.get("contract").and_then(Value::as_str) == Some(kind.contract_name());
        let (pointer, message) = if names_contract {
            let latest = kind.latest_version();
            (
                "/contract_version",
                format!(
                    "supported {kind} versions are {}.0.0 to {latest}",
                    latest.major
                ),
            )
        } else {
            (
                "/contract",
                format!("the document must have `contract` equal to `{kind}`"),
            )
        };
        Err(self.error(self.unsupported_code(), pointer, message))
    }

    /// Validates the document against its JSON Schema.
    ///
    /// # Errors
    ///
    /// [`Self::invalid_code`], located at the first violation.
    pub fn validate(self, document: &Value) -> Result<(), AdmissionError> {
        validate_structure(self.kind(), document).map_err(|violation| {
            self.error(self.invalid_code(), &violation.pointer, violation.message)
        })
    }

    pub(crate) fn error(
        self,
        code: AdmissionCode,
        pointer: &str,
        message: String,
    ) -> AdmissionError {
        AdmissionError {
            code,
            document: self.document(),
            pointer: pointer.to_owned(),
            line_column: None,
            message,
        }
    }
}

/// Checks that a query has exactly one of `query` and `sql` (QRY-2).
///
/// # Errors
///
/// `EQRY-01`, located at the document root.
pub fn check_query_form(document: &Value) -> Result<(), AdmissionError> {
    let present = ["query", "sql"]
        .iter()
        .filter(|name| document.get(**name).is_some())
        .count();
    if present == 1 {
        return Ok(());
    }
    Err(IncomingContract::Query.error(
        AdmissionCode::Eqry01,
        "",
        "a query must have exactly one of `query` and `sql`".to_owned(),
    ))
}

/// Converts a document that satisfies its JSON Schema into its Rust type.
///
/// Numbers with a zero fractional part are integers for JSON Schema; they are read as integers.
///
/// # Errors
///
/// [`IncomingContract::invalid_code`] when the document does not have the shape of its schema.
pub fn decode<T: ContractDocument>(
    contract: IncomingContract,
    document: &Value,
) -> Result<T, AdmissionError> {
    serde_json::from_value(integral_numbers_as_integers(document)).map_err(|error| {
        contract.error(
            contract.invalid_code(),
            "",
            format!("the document does not match its schema: {error}"),
        )
    })
}

fn integral_numbers_as_integers(value: &Value) -> Value {
    match value {
        Value::Number(number) => {
            Value::Number(as_integer(number).unwrap_or_else(|| number.clone()))
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(integral_numbers_as_integers).collect())
        }
        Value::Object(members) => Value::Object(
            members
                .iter()
                .map(|(name, member)| (name.clone(), integral_numbers_as_integers(member)))
                .collect::<Map<String, Value>>(),
        ),
        other => other.clone(),
    }
}

/// Converts `1.0` to `1` when the integer is exactly representable as `i64`.
fn as_integer(number: &Number) -> Option<Number> {
    let value = number.as_f64().filter(|_| number.is_f64())?;
    let limit = 2f64.powi(53);
    if value.fract() == 0.0 && value.abs() < limit {
        format!("{value:.0}").parse::<i64>().ok().map(Number::from)
    } else {
        None
    }
}
