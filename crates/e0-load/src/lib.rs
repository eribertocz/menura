//! Stage E0: loads and validates a semantic model.
//!
//! Until the E0 stage function exists (#45), the crate exports nothing: its substages are
//! private (S9), and diagnostics will reach callers through the compile error document.

#![forbid(unsafe_code)]

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod classification;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod columns;
mod expressions;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod extensions;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod names;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod nodes;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod outline;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod pointer;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod read;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod references;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod relationships;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod structure;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "called by the E0 stage function, which issue #45 adds; until then only the tests call it"
    )
)]
mod typing;
