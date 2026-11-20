//! Types one expression tree (E0F-3 to E0F-11): finds its result type and whether it can be
//! NULL (E0F-5, E0F-5a, E0F-7.1), or the first typing problem in written order (E0F-8).
//!
//! The typer knows nothing about where the expression comes from: its [`TypingScope`] says
//! what each reference, `AGG(metric)` and `USER_ATTRIBUTE('name')` is, and whether an
//! aggregate may appear (E0F-07). The same expression and scope always give the same result.
//!
//! ```
//! use menura_contracts::types::common::LogicalType;
//! use menura_expressions::parse;
//! use menura_expressions::typing::{
//!     Aggregates, Stop, Typed, TypingScope, big_integer, type_expression,
//! };
//! use menura_model::expression::Reference;
//!
//! let resolve = |_: &Reference| -> Result<Typed, Stop> {
//!     Ok(Typed::value(LogicalType::Text, true))
//! };
//! let scope = TypingScope::new(&resolve, Aggregates::NotAllowed);
//! let typed = type_expression(&parse("LENGTH(orders.status)").unwrap(), &scope).unwrap();
//! assert_eq!(typed, Typed::value(big_integer(), true));
//! ```

mod aggregates;
mod arithmetic;
mod code;
mod conversion;
mod dates;
mod functions;
mod numbers;
mod operators;
mod stop;
mod typer;
mod value_type;

pub use code::TypingCode;
pub use stop::{Outcome, Stop, problem};
pub use typer::{Aggregates, AttributeResolver, MetricAggregates, Resolver, TypingScope};
pub use value_type::{
    ExpressionType, Typed, big_integer, decimal, describe, double, huge_integer, integer, timestamp,
};

use menura_model::expression::Expression;

/// The type and nullability of `expression` in `scope`, or why it has none: the first typing
/// problem in written order, with its code (E0F-8), or [`Stop::ReportedElsewhere`] when it
/// reads something the scope leaves to another check.
///
/// # Errors
///
/// [`Stop::Problem`] for a typing problem (`E0F-02` to `E0F-08`), and
/// [`Stop::ReportedElsewhere`] when the scope gives a reference no type.
pub fn type_expression(expression: &Expression, scope: &TypingScope<'_>) -> Outcome<Typed> {
    scope.type_of(expression)
}
