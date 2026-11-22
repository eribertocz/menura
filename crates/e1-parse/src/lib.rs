//! Stage E1: parses a query document into the query tree (E1-1).
//!
//! The crate exports only the stage function, [`parse_query`], its result type,
//! [`ParsedQuery`], its error and warning types, [`ParseError`] and [`ParseWarning`] (S9), and
//! [`time_zone_database_version`], the version of the bundled time-zone database (E6-5.5).

#![forbid(unsafe_code)]

mod envelope;
mod error;
mod filters;
mod pointer;
mod reference_time;
mod references;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the stage reports E_NOT_YET_SUPPORTED for the SQL form until the 0.5.0 slice \
                  issue removes that check (VS-2); only the unit tests reach the parser"
    )
)]
mod sql;
mod stage;
mod time_zone;
mod time_zone_database;
mod tree;
mod warnings;

pub use error::{ParseError, ParseErrorCode, ParseWarning};
pub use stage::{ParsedQuery, parse_query};
pub use time_zone_database::time_zone_database_version;
