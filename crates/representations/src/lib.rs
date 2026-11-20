//! Data types passed between compilation stages (S9).

#![forbid(unsafe_code)]

pub mod bound_query;
pub mod columns;
pub mod logical_plan;
pub mod names;
pub mod origin;
pub mod paths;
pub mod query_tree;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
