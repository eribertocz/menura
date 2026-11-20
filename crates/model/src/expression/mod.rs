//! The normalized expression tree (E0H-1.3).
//!
//! The crate `menura-expressions` is the only producer of these trees from
//! expression text (S10). A tree records meaning, never format: whitespace, keyword case and
//! parentheses leave no trace in it, so formulas that differ only in those produce equal trees.
//!
//! These types hold data and the constructors that keep it valid; they carry no parsing,
//! resolution or typing logic (S9).

mod call;
mod cast;
mod json_form;
mod literal;
mod name;
mod operator;
mod tree;

pub use call::{
    AggregateCall, DateUnit, DistinctAggregate, ExtractPart, PercentileFunction, ScalarFunction,
    SimpleAggregate, SortDirection,
};
pub use cast::{CastTarget, DecimalTarget};
pub use literal::{
    DateValue, DecimalLiteral, FloatLiteral, IntegerLiteral, Literal, MAXIMUM_EXACT_DIGITS,
    TimeOfDay, TimestampValue,
};
pub use name::{Identifier, Reference};
pub use operator::{BinaryOperator, LikeCase, UnaryOperator};
pub use tree::{CaseBranch, Expression};
