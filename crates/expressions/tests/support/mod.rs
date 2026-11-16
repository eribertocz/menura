//! Builders that keep expected trees short in the tests.

#![allow(
    dead_code,
    reason = "each test file uses a different part of the builders"
)]

pub mod generator;

use semantic_compiler_expressions::{ExpressionCode, parse};
use semantic_compiler_model::expression::{
    BinaryOperator, ExactNumber, Expression, Identifier, Literal, Reference,
};

pub fn column(name: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::regular(name).expect("regular identifier"),
    })
}

pub fn field(dataset: &str, name: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: Some(Identifier::regular(dataset).expect("regular identifier")),
        name: Identifier::regular(name).expect("regular identifier"),
    })
}

pub fn integer(digits: &str) -> Expression {
    Expression::Literal(Literal::Exact(
        ExactNumber::new(digits, "").expect("digits"),
    ))
}

pub fn decimal(integer_digits: &str, fraction_digits: &str) -> Expression {
    Expression::Literal(Literal::Exact(
        ExactNumber::new(integer_digits, fraction_digits).expect("digits"),
    ))
}

pub fn text(value: &str) -> Expression {
    Expression::Literal(Literal::Text(value.to_owned()))
}

pub fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// Parses `text`, panicking with the error when it fails.
pub fn tree(text: &str) -> Expression {
    match parse(text) {
        Ok(tree) => tree,
        Err(error) => panic!("{text:?} should parse: {error}"),
    }
}

/// The code of the error `text` gives, panicking when it parses.
pub fn code_of(text: &str) -> ExpressionCode {
    match parse(text) {
        Ok(tree) => panic!("{text:?} should fail, parsed as {tree:?}"),
        Err(error) => error.code(),
    }
}
