//! The Python package `menura`: a wrapper around the core, without compilation logic of its own
//! (ARC-6.3, A4).
//!
//! Each call converts its arguments (ARC-6.4), calls `core` with the interpreter lock released
//! (ARC-6.5a) and returns the core's documents as a [`result::CallResult`] (ARC-6.5). The
//! module functions are `validate`, `migrate` and `diff`; the class `Handle` holds a loaded
//! model; `CompileError` is raised when `Handle.load` cannot load one.

#![forbid(unsafe_code)]

mod arguments;
mod errors;
mod functions;
mod handle;
mod options;
mod result;

use pyo3::prelude::*;

/// The module `menura`.
#[pymodule]
fn menura(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(functions::validate, module)?)?;
    module.add_function(wrap_pyfunction!(functions::migrate, module)?)?;
    module.add_function(wrap_pyfunction!(functions::diff, module)?)?;
    module.add_class::<handle::PythonHandle>()?;
    module.add_class::<result::CallResult>()?;
    module.add(
        "CompileError",
        module.py().get_type::<errors::CompileError>(),
    )?;
    Ok(())
}
