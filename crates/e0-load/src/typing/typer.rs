//! Types one expression tree (E0F-3 to E0F-7): finds its result type and nullability, or the
//! first typing problem in written order.

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::{
    CastTarget, Expression, IntegerLiteral, Literal, Reference,
};

use super::diagnostic::TypingCode;
use super::stop::{Outcome, Stop, incompatible, not_allowed, problem};
use super::value_type::{
    ExpressionType, Numeric, Typed, big_integer, decimal, describe, double, integer, numeric,
    timestamp,
};

/// Finds the type of a reference: a catalog column in a field expression, a field in a metric
/// expression.
pub(crate) type Resolver<'resolver> = dyn Fn(&Reference) -> Outcome<Typed> + 'resolver;

/// Whether an aggregate is allowed where the typer is (E0F-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Aggregates {
    /// At the top of a metric expression: outside any aggregate.
    Allowed,
    /// In a field expression.
    NotAllowed,
    /// Inside an aggregate's argument or `FILTER`.
    Nested,
    /// In a row-level condition of an extension, such as a population `required_when`
    /// (POP-4.4): no aggregate, `AGG(metric)` or `USER_ATTRIBUTE` may appear.
    Condition,
}

/// Types the expressions of one element.
pub(crate) struct ExpressionTyper<'resolver> {
    resolve: &'resolver Resolver<'resolver>,
    pub(super) aggregates: Aggregates,
}

impl<'resolver> ExpressionTyper<'resolver> {
    /// A typer that resolves references with `resolve`, where `aggregates` says whether an
    /// aggregate may appear.
    pub(crate) fn new(resolve: &'resolver Resolver<'resolver>, aggregates: Aggregates) -> Self {
        Self {
            resolve,
            aggregates,
        }
    }

    /// The typer of an aggregate's argument and `FILTER`, where no aggregate may appear.
    pub(super) fn nested(&self) -> Self {
        Self::new(self.resolve, Aggregates::Nested)
    }

    /// The type and nullability of `expression`.
    pub(crate) fn type_of(&self, expression: &Expression) -> Outcome<Typed> {
        match expression {
            Expression::Literal(literal) => Ok(literal_type(literal)),
            Expression::Reference(reference) => (self.resolve)(reference),
            Expression::MetricAggregate(_) | Expression::UserAttribute(_)
                if self.aggregates == Aggregates::Condition =>
            {
                not_allowed("AGG(metric) or USER_ATTRIBUTE in a row-level condition".to_owned())
            }
            Expression::MetricAggregate(_) | Expression::UserAttribute(_) => {
                Err(Stop::ReportedElsewhere)
            }
            Expression::CurrentDate => Ok(Typed::value(LogicalType::Date, false)),
            Expression::CurrentTime => Ok(Typed::value(LogicalType::Time, false)),
            Expression::CurrentTimestamp => Ok(Typed::value(timestamp(), false)),
            Expression::Cast { operand, target } => {
                let operand = self.type_of(operand)?;
                Ok(Typed::value(cast_type(*target), operand.nullable))
            }
            Expression::Function {
                function,
                arguments,
            } => self.function(*function, arguments),
            Expression::Aggregate { call, filter } => self.aggregate(call, filter.as_deref()),
            Expression::Extract { .. }
            | Expression::DateTrunc { .. }
            | Expression::DateAdd { .. }
            | Expression::DateDiff { .. } => self.date_function(expression),
            Expression::Unary { .. }
            | Expression::Binary { .. }
            | Expression::IsNull { .. }
            | Expression::IsDistinctFrom { .. }
            | Expression::InList { .. }
            | Expression::Between { .. }
            | Expression::Like { .. }
            | Expression::Case { .. } => self.operator(expression),
        }
    }

    /// The types of `expressions`, in order.
    pub(super) fn types_of(&self, expressions: &[Expression]) -> Outcome<Vec<Typed>> {
        expressions
            .iter()
            .map(|expression| self.type_of(expression))
            .collect()
    }

    /// Types `expression`, which must be boolean (E0F-06), and returns whether it can be NULL.
    pub(super) fn condition(&self, expression: &Expression, role: &str) -> Outcome<bool> {
        let typed = self.type_of(expression)?;
        match typed.logical() {
            None | Some(LogicalType::Boolean) => Ok(typed.nullable),
            Some(_) => problem(
                TypingCode::NotBoolean,
                format!(
                    "{role} must be boolean, not {}",
                    describe(&typed.value_type)
                ),
            ),
        }
    }
}

/// The numeric kind of `typed`, `None` for the literal `NULL`; `E0F-03` for any other type.
pub(super) fn expect_numeric(typed: &Typed, role: &str) -> Outcome<Option<Numeric>> {
    let Some(value_type) = typed.logical() else {
        return Ok(None);
    };
    match numeric(value_type) {
        Some(found) => Ok(Some(found)),
        None => incompatible(format!(
            "{role} must be numeric, not {}",
            describe(&typed.value_type)
        )),
    }
}

/// `E0F-03` unless `typed` is a type that `accepts` takes, or the literal `NULL`; `name` names
/// the accepted types for the message.
pub(super) fn expect_type(
    typed: &Typed,
    accepts: impl Fn(&LogicalType) -> bool,
    role: &str,
    name: &str,
) -> Outcome<()> {
    match typed.logical() {
        Some(value_type) if !accepts(value_type) => incompatible(format!(
            "{role} must be {name}, not {}",
            describe(&typed.value_type)
        )),
        _ => Ok(()),
    }
}

/// `E0F-03` unless `typed` is text or the literal `NULL`.
pub(super) fn expect_text(typed: &Typed, role: &str) -> Outcome<()> {
    expect_type(
        typed,
        |value_type| *value_type == LogicalType::Text,
        role,
        "text",
    )
}

/// Whether any of `typed` can be NULL.
pub(super) fn any_nullable(typed: &[Typed]) -> bool {
    typed.iter().any(|typed| typed.nullable)
}

/// The type of a literal (E0F-9.3).
fn literal_type(literal: &Literal) -> Typed {
    let value_type = match literal {
        Literal::Null => {
            return Typed {
                value_type: ExpressionType::Null,
                nullable: true,
            };
        }
        Literal::Boolean(_) => LogicalType::Boolean,
        Literal::Text(_) => LogicalType::Text,
        Literal::Integer(number) => {
            let digits = number.digits().len();
            if digits <= IntegerLiteral::MAXIMUM_BIG_INTEGER_DIGITS {
                big_integer()
            } else {
                decimal(digits_count(digits), 0)
            }
        }
        Literal::Decimal(number) => decimal(
            digits_count(number.precision()),
            digits_count(number.scale()),
        ),
        Literal::Float(_) => double(),
        Literal::Date(_) => LogicalType::Date,
        Literal::Time(_) => LogicalType::Time,
        Literal::Timestamp(_) => timestamp(),
    };
    Typed::value(value_type, false)
}

/// A digit count of a literal, at most 38 by construction.
fn digits_count(digits: usize) -> u8 {
    u8::try_from(digits).unwrap_or(u8::MAX)
}

/// The type of a `CAST` target (E0F-11).
fn cast_type(target: CastTarget) -> LogicalType {
    match target {
        CastTarget::Text => LogicalType::Text,
        CastTarget::Integer => integer(),
        CastTarget::BigInteger => big_integer(),
        CastTarget::Decimal(target) => decimal(target.precision(), target.scale()),
        CastTarget::Double => double(),
        CastTarget::Boolean => LogicalType::Boolean,
        CastTarget::Date => LogicalType::Date,
        CastTarget::Time => LogicalType::Time,
        CastTarget::Timestamp => timestamp(),
    }
}
