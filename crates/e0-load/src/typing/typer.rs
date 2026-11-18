//! Types one expression tree (E0F-3 to E0F-7): finds its result type and nullability, or the
//! first typing problem in written order.

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::{
    CastTarget, Expression, IntegerLiteral, Literal, Reference,
};

use super::diagnostic::TypingCode;
use super::stop::{Outcome, Stop, incompatible, problem};
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
}

/// What `AGG(metric)` is where the typer is (DER-5.1.2).
#[derive(Clone, Copy)]
pub(crate) enum MetricAggregates<'resolver> {
    /// In a model metric: whether it may read a metric is checked by the derived-metric
    /// extension (`E0C-37`), and a derived formula is typed by its own typer. The expression
    /// gets no type here.
    ReportedElsewhere,
    /// In a field or a dataset metric, where no metric may be read: `E0F-02`.
    OutsideSubset,
    /// In a derived formula: `AGG(m)` has the type and nullability of `m` (DER-5.1.3), which
    /// this resolver finds.
    Typed(&'resolver Resolver<'resolver>),
}

/// Types the expressions of one element.
pub(crate) struct ExpressionTyper<'resolver> {
    resolve: &'resolver Resolver<'resolver>,
    pub(super) aggregates: Aggregates,
    metric_aggregates: MetricAggregates<'resolver>,
}

impl<'resolver> ExpressionTyper<'resolver> {
    /// A typer that resolves references with `resolve`, where `aggregates` says whether an
    /// aggregate may appear. Where no aggregate is allowed, `AGG(…)` is `E0F-02`; elsewhere it
    /// is reported by the derived-metric checks, unless [`Self::with_metric_aggregates`] says
    /// otherwise.
    pub(crate) fn new(resolve: &'resolver Resolver<'resolver>, aggregates: Aggregates) -> Self {
        let metric_aggregates = match aggregates {
            Aggregates::NotAllowed => MetricAggregates::OutsideSubset,
            Aggregates::Allowed | Aggregates::Nested => MetricAggregates::ReportedElsewhere,
        };
        Self {
            resolve,
            aggregates,
            metric_aggregates,
        }
    }

    /// The same typer, where `AGG(…)` is `metric_aggregates`.
    pub(crate) fn with_metric_aggregates(
        self,
        metric_aggregates: MetricAggregates<'resolver>,
    ) -> Self {
        Self {
            metric_aggregates,
            ..self
        }
    }

    /// The typer of an aggregate's argument and `FILTER`, where no aggregate may appear.
    pub(super) fn nested(&self) -> Self {
        Self {
            resolve: self.resolve,
            aggregates: Aggregates::Nested,
            metric_aggregates: self.metric_aggregates,
        }
    }

    /// The type and nullability of `expression`.
    pub(crate) fn type_of(&self, expression: &Expression) -> Outcome<Typed> {
        match expression {
            Expression::Literal(literal) => Ok(literal_type(literal)),
            Expression::Reference(reference) => (self.resolve)(reference),
            Expression::MetricAggregate(metric) => self.metric_aggregate(metric),
            Expression::UserAttribute(_) => Err(Stop::ReportedElsewhere),
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

    /// The type of `AGG(metric)` (DER-5.1.2, DER-5.1.3).
    fn metric_aggregate(&self, metric: &Reference) -> Outcome<Typed> {
        match self.metric_aggregates {
            MetricAggregates::ReportedElsewhere => Err(Stop::ReportedElsewhere),
            MetricAggregates::OutsideSubset => problem(
                TypingCode::OutsideSubset,
                format!(
                    "AGG({}) reads a metric, which only a model metric may do (DER-5.1.2)",
                    metric.name.text()
                ),
            ),
            MetricAggregates::Typed(resolve) => resolve(metric),
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
