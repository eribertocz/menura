//! The load options of architecture section 3.5, and their JSON object form (ARC-5.5).

use std::fmt;

use menura_contracts::types::audit_record::{self, OssieFormat};
use serde_json::Value;

/// The default extension namespace (MDL-2.3).
const DEFAULT_NAMESPACE: &str = "menura";

/// The options `load` and `validate` read (architecture section 3.5).
///
/// # Example
///
/// ```
/// use menura_core::{LoadOptions, OssieFormat};
///
/// let options = LoadOptions::from_json(br#"{"ossie_format": "development"}"#)?;
/// assert_eq!(options.ossie_format, OssieFormat::Development);
/// assert_eq!(options.namespace, "menura");
/// # Ok::<(), menura_core::LoadOptionsError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOptions {
    /// Which semantic model to load when the document has several (DEV-7).
    pub model_name: Option<String>,
    /// `release` or `development` (section 4.5); `release` by default.
    pub ossie_format: OssieFormat,
    /// Treat datasets without `access` as `authenticated` (DEV-4); `false` by default.
    pub grant_authenticated_to_all: bool,
    /// The extension namespace (MDL-2.3); `menura` by default.
    pub namespace: String,
    /// The logical source name written in every plan; the catalog schema
    /// `document_id.origin` when `None`.
    pub data_source: Option<String>,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            model_name: None,
            ossie_format: OssieFormat::Release,
            grant_authenticated_to_all: false,
            namespace: DEFAULT_NAMESPACE.to_owned(),
            data_source: None,
        }
    }
}

/// A load-options document that is not a JSON object of load options (ARC-5.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOptionsError {
    message: String,
}

impl LoadOptionsError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for LoadOptionsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LoadOptionsError {}

impl LoadOptions {
    /// Reads the load options from `bytes`, a UTF-8 JSON object whose members are load
    /// options; an option it omits takes its default (ARC-5.5).
    ///
    /// # Errors
    ///
    /// A [`LoadOptionsError`] when `bytes` is not UTF-8 JSON, is not an object, has a member
    /// that is not a load option, or has a value of the wrong type (`null` included).
    pub fn from_json(bytes: &[u8]) -> Result<Self, LoadOptionsError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| LoadOptionsError::new("the load options are not UTF-8"))?;
        let value: Value = serde_json::from_str(text).map_err(|error| {
            LoadOptionsError::new(format!("the load options are not JSON: {error}"))
        })?;
        let Value::Object(members) = value else {
            return Err(LoadOptionsError::new(
                "the load options are not a JSON object",
            ));
        };
        let mut options = Self::default();
        for (name, value) in &members {
            options.set(name, value)?;
        }
        Ok(options)
    }

    fn set(&mut self, name: &str, value: &Value) -> Result<(), LoadOptionsError> {
        match name {
            "model_name" => self.model_name = Some(text(name, value)?),
            "ossie_format" => self.ossie_format = format(value)?,
            "grant_authenticated_to_all" => self.grant_authenticated_to_all = boolean(name, value)?,
            "namespace" => self.namespace = text(name, value)?,
            "data_source" => self.data_source = Some(text(name, value)?),
            _ => {
                return Err(LoadOptionsError::new(format!(
                    "`{name}` is not a load option; the options are `model_name`, `ossie_format`, \
                     `grant_authenticated_to_all`, `namespace` and `data_source`"
                )));
            }
        }
        Ok(())
    }

    /// The options the E0 stage function reads.
    pub(crate) fn stage_options(&self) -> audit_record::LoadOptions {
        audit_record::LoadOptions {
            grant_authenticated_to_all: self.grant_authenticated_to_all,
            ossie_format: self.ossie_format,
            namespace: self.namespace.clone(),
            model_name: self.model_name.clone(),
            data_source: None,
        }
    }
}

fn text(name: &str, value: &Value) -> Result<String, LoadOptionsError> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| LoadOptionsError::new(format!("the load option `{name}` must be a string")))
}

fn boolean(name: &str, value: &Value) -> Result<bool, LoadOptionsError> {
    value
        .as_bool()
        .ok_or_else(|| LoadOptionsError::new(format!("the load option `{name}` must be a boolean")))
}

fn format(value: &Value) -> Result<OssieFormat, LoadOptionsError> {
    match value.as_str() {
        Some("release") => Ok(OssieFormat::Release),
        Some("development") => Ok(OssieFormat::Development),
        _ => Err(LoadOptionsError::new(
            "the load option `ossie_format` must be `release` or `development`",
        )),
    }
}
