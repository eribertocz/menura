//! Stage E0: loads and validates a semantic model.
//!
//! The crate exports only the stage function, [`load_model`], its result type,
//! [`LoadedModel`], and its error type, [`LoadRejection`] (S9); its substages E0.a to E0.h are
//! private.

#![forbid(unsafe_code)]

mod classification;
mod columns;
mod expressions;
mod extensions;
mod internal_model;
mod names;
mod nodes;
mod outline;
mod pointer;
mod read;
mod references;
mod relationships;
mod stage;
mod structure;
mod typing;

pub use stage::{LoadRejection, LoadedModel, load_model};
