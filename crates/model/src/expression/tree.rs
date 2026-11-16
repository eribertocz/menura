//! The expression tree itself.

use super::call::{AggregateCall, DateUnit, ExtractPart, ScalarFunction};
use super::cast::CastTarget;
use super::literal::Literal;
use super::name::Reference;
use super::operator::{BinaryOperator, LikeCase, UnaryOperator};

/// One node of a normalized expression tree (E0H-1.3).
///
/// Every variant is a construct of the accepted subset of E0F-2.2. Where the language offers
/// two spellings of the same construct (`NUMERIC` and `DECIMAL`, `ASC` and no direction), the
/// tree has one form.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression {
    /// A constant.
    Literal(Literal),
    /// A column or field reference: `column` or `dataset.field`.
    Reference(Reference),
    /// `AGG(metric)`: the value of another metric at the query grain.
    MetricAggregate(Reference),
    /// `USER_ATTRIBUTE('name')`: an attribute of the security context.
    UserAttribute(String),
    /// `CURRENT_DATE` or `CURRENT_DATE()`, resolved at compilation from the query's reference time (E0F-2.3).
    CurrentDate,
    /// `CURRENT_TIME`, resolved at compilation from the query's reference time (E0F-2.3).
    CurrentTime,
    /// `CURRENT_TIMESTAMP`, resolved at compilation from the query's reference time (E0F-2.3).
    CurrentTimestamp,
    /// A prefix operator.
    Unary {
        /// The operator.
        operator: UnaryOperator,
        /// Its operand.
        operand: Box<Expression>,
    },
    /// An infix operator.
    Binary {
        /// The operator.
        operator: BinaryOperator,
        /// The left operand.
        left: Box<Expression>,
        /// The right operand.
        right: Box<Expression>,
    },
    /// `operand IS [NOT] NULL`.
    IsNull {
        /// The tested value.
        operand: Box<Expression>,
        /// `true` for `IS NOT NULL`.
        negated: bool,
    },
    /// `left IS [NOT] DISTINCT FROM right`.
    IsDistinctFrom {
        /// The left operand.
        left: Box<Expression>,
        /// The right operand.
        right: Box<Expression>,
        /// `true` for `IS NOT DISTINCT FROM`.
        negated: bool,
    },
    /// `operand [NOT] IN (list)`.
    InList {
        /// The tested value.
        operand: Box<Expression>,
        /// The candidate values, at least one.
        list: Vec<Expression>,
        /// `true` for `NOT IN`.
        negated: bool,
    },
    /// `operand [NOT] BETWEEN low AND high`.
    Between {
        /// The tested value.
        operand: Box<Expression>,
        /// The lower bound.
        low: Box<Expression>,
        /// The upper bound.
        high: Box<Expression>,
        /// `true` for `NOT BETWEEN`.
        negated: bool,
    },
    /// `operand [NOT] LIKE pattern` or `operand [NOT] ILIKE pattern`.
    Like {
        /// The tested value.
        operand: Box<Expression>,
        /// The pattern.
        pattern: Box<Expression>,
        /// `LIKE` or `ILIKE`.
        case: LikeCase,
        /// `true` for `NOT LIKE` and `NOT ILIKE`.
        negated: bool,
    },
    /// `CASE [operand] WHEN … THEN … [ELSE …] END`.
    Case {
        /// The compared value of a simple `CASE`; `None` for a searched `CASE`.
        operand: Option<Box<Expression>>,
        /// The `WHEN` branches, at least one, in order.
        branches: Vec<CaseBranch>,
        /// The `ELSE` result, if written.
        otherwise: Option<Box<Expression>>,
    },
    /// `CAST(operand AS target)`.
    Cast {
        /// The converted value.
        operand: Box<Expression>,
        /// The target type.
        target: CastTarget,
    },
    /// `EXTRACT(part FROM operand)`, also written `DATE_PART('part', operand)`.
    Extract {
        /// The extracted part.
        part: ExtractPart,
        /// The date, time or timestamp.
        operand: Box<Expression>,
    },
    /// `DATE_TRUNC('unit', operand)`.
    DateTrunc {
        /// The unit truncated to.
        unit: DateUnit,
        /// The date or timestamp.
        operand: Box<Expression>,
    },
    /// `DATEADD(unit, amount, operand)`, with the unit as a bare word or a text literal.
    DateAdd {
        /// The unit added.
        unit: DateUnit,
        /// The number of units.
        amount: Box<Expression>,
        /// The date or timestamp.
        operand: Box<Expression>,
    },
    /// `DATEDIFF(unit, start, end)`, with the unit as a bare word or a text literal.
    DateDiff {
        /// The unit counted.
        unit: DateUnit,
        /// The start.
        start: Box<Expression>,
        /// The end.
        end: Box<Expression>,
    },
    /// A scalar function call of E0F-10 with a plain argument list.
    Function {
        /// The function.
        function: ScalarFunction,
        /// Its arguments, in order; their number matches the function.
        arguments: Vec<Expression>,
    },
    /// An aggregate call, with its optional `FILTER (WHERE …)`.
    Aggregate {
        /// The call.
        call: AggregateCall,
        /// The `FILTER (WHERE …)` condition, if written.
        filter: Option<Box<Expression>>,
    },
}

/// One `WHEN … THEN …` branch of a `CASE`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CaseBranch {
    /// The condition, or the compared value of a simple `CASE`.
    pub condition: Expression,
    /// The result when the branch matches.
    pub result: Expression,
}
