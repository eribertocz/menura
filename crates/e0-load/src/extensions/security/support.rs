//! The helpers the tests of the security extension in E0.c share.

use menura_contracts::types::audit_record::OssieFormat;

use super::fixture::{ORDERS_DATA, with_access};
use crate::extensions::output::{Extensions, ExtensionsFailure};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::structure::validate_structure;

pub(super) fn validate_in(
    model: &str,
    format: OssieFormat,
) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

pub(super) fn validate(model: &str) -> Result<Extensions, ExtensionsFailure> {
    validate_in(model, OssieFormat::Release)
}

/// The code and JSON Pointer of every rejecting diagnostic of `model`.
pub(super) fn rejected(model: &str) -> Vec<(&'static str, String)> {
    validate(model)
        .expect_err("the extensions are rejected")
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

/// The codes that reject `model`, each at the `data` member `data`.
pub(super) fn codes_at(model: &str, data: &str) -> Vec<&'static str> {
    rejected(model)
        .into_iter()
        .map(|(code, pointer)| {
            assert_eq!(pointer, data);
            code
        })
        .collect()
}

pub(super) fn access_codes(access: &str) -> Vec<&'static str> {
    codes_at(&with_access(access), ORDERS_DATA)
}
