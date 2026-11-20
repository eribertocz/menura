//! Stage E1: parses a query document into the query tree (E1-1).
//!
//! The crate exports only the stage function, [`parse_query`], its result type,
//! [`ParsedQuery`], and its error and warning types, [`ParseError`] and [`ParseWarning`] (S9).

#![forbid(unsafe_code)]

mod envelope;
mod error;
mod filters;
mod pointer;
mod reference_time;
mod references;
mod stage;
mod time_zone;
mod tree;
mod warnings;

pub use error::{ParseError, ParseErrorCode, ParseWarning};
pub use stage::{ParsedQuery, parse_query};
