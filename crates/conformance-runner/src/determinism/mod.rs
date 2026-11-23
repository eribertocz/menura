//! The determinism gate (PLN-2.4, P3): every conformance case compiled in two separate
//! processes, the second with the keys of every input document reordered with a fixed seed,
//! gives byte-identical documents and hashes.
//!
//! [`run_calls`] computes, through `core`, every wrapper call of the suite (plans, compile
//! errors, descriptions, explanations, integrity checks, audit records and the other
//! commands), after [`reorder_document`] when asked; the `determinism` binary runs it in two
//! child processes and [`compare_runs`] reports each call whose documents differ. Parts that
//! the specification derives from the text of an input (positions, document order, the model
//! `migrate` returns) are set aside by [`equal_up_to_input_order`] and counted apart.

mod block;
mod document;
mod flow;
mod input_order;
mod permutation;
mod run;

pub use document::reorder_document;
pub use input_order::equal_up_to_input_order;
pub use permutation::{Permutation, SEED};
pub use run::{CallOutput, Comparison, RunDifference, RunOutputs, compare_runs, run_calls};
