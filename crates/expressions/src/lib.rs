//! Turns expression text into the normalized expression tree of the model (S10).
//!
//! [`parse`] reads `ANSI_SQL` expression text (E0F-2.1), accepts exactly the subset of
//! E0F-2.2 and returns the normalized tree of E0H-1.3; [`print`] writes a tree back as
//! canonical text that parses to the same tree.
//!
//! ```
//! use menura_expressions::{parse, print};
//! let tree = parse("sum ( (orders.amount) )").unwrap();
//! assert_eq!(tree, parse("SUM(orders.amount)").unwrap());
//! assert_eq!(print(&tree), "SUM(orders.amount)");
//! ```

#![forbid(unsafe_code)]

mod error;
mod lexer;
mod parser;
mod printer;
mod vocabulary;

pub use error::{ExpressionCode, ExpressionError, ExpressionErrorKind, Span};
pub use parser::parse;
pub use printer::print;
