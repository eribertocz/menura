//! Command-line wrapper around the core, without compilation logic of its own.

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the command-line wrapper is the only crate allowed to use I/O, the environment and the clock"
)]

fn main() {}
