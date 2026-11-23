//! The document arguments: `bytes` passed unchanged, or `str` encoded as UTF-8 (ARC-6.4).

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

/// A document argument, as the bytes `core` receives.
pub(crate) struct Document(pub(crate) Vec<u8>);

impl<'a, 'py> FromPyObject<'a, 'py> for Document {
    type Error = PyErr;

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(bytes) = object.cast::<PyBytes>() {
            return Ok(Self(bytes.as_bytes().to_vec()));
        }
        let text = object.cast::<PyString>()?;
        Ok(Self(text.to_str()?.as_bytes().to_vec()))
    }
}

/// The bytes of an optional document argument; `None` is absent (ARC-6.4).
pub(crate) fn optional_bytes(document: Option<Document>) -> Option<Vec<u8>> {
    document.map(|Document(bytes)| bytes)
}
