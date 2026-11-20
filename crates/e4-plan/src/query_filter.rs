//! The query-filter predicate: the query filters as one conjunction, followed by the
//! `date_range` predicate (E4-4 step 4, QRY-4.2).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{BinaryOperator, DateValue, Expression, Literal};
use menura_model::internal_model::{Dataset, ValueType};
use menura_representations::bound_query::{BoundDateRange, BoundFilter, BoundQuery};
use menura_representations::origin::QueryLocation;

use crate::columns::over_scan;
use crate::dimensions::{at, home_field_column};
use crate::error::{InvariantViolation, PlanError};

/// The predicate of the query-filter `Filter` over the `Scan` columns of `home`, with the
/// element it comes from: the first filter, or the date range. `None` without filters and
/// date range.
///
/// # Errors
///
/// An internal error for a filter or date range outside `home`, or a date range the plan
/// cannot write.
pub(crate) fn query_predicate(
    query: &BoundQuery,
    home: &Dataset,
) -> Result<Option<(Expression, QueryLocation)>, PlanError> {
    let mut terms = Vec::new();
    for filter in query.filters() {
        terms.push(filter_term(filter, home).map_err(|violation| at(violation, &filter.origin))?);
    }
    if let Some(range) = query.date_range() {
        terms.push(date_range_term(range, home).map_err(|violation| at(violation, &range.origin))?);
    }
    let origin = query
        .filters()
        .first()
        .map(|filter| &filter.origin)
        .or_else(|| query.date_range().map(|range| &range.origin));
    let predicate = terms.into_iter().reduce(and);
    Ok(predicate.zip(origin.cloned()))
}

fn filter_term(filter: &BoundFilter, home: &Dataset) -> Result<Expression, InvariantViolation> {
    if filter.datasets.iter().any(|dataset| dataset != &home.name) {
        return Err(InvariantViolation::ForeignField);
    }
    over_scan(&filter.expression, false)
}

/// `x >= from AND x <= to` for a date field; `x >= from AND x < to + 1 day` for a timestamp
/// field, with `date` literals that E6 converts to the start of that day (E6-4.2b). When `to`
/// is 9999-12-31, a timestamp field has no upper bound (QRY-4.2).
fn date_range_term(
    range: &BoundDateRange,
    home: &Dataset,
) -> Result<Expression, InvariantViolation> {
    let column = home_field_column(&range.field, home)?;
    let field_type = home
        .field(&range.field.field)
        .and_then(|field| field.typed.as_ref())
        .map(|typed| &typed.value_type);
    let lower = comparison(BinaryOperator::GreaterOrEqual, column.clone(), range.from);
    let upper = match field_type {
        Some(ValueType::Logical(LogicalType::Date)) => {
            Some(comparison(BinaryOperator::LessOrEqual, column, range.to))
        }
        Some(ValueType::Logical(LogicalType::Timestamp { .. })) => {
            next_day(range.to).map(|end| comparison(BinaryOperator::Less, column, end))
        }
        _ => return Err(InvariantViolation::UnplannableDateRange),
    };
    Ok(match upper {
        Some(upper) => and(lower, upper),
        None => lower,
    })
}

fn comparison(operator: BinaryOperator, column: Expression, date: DateValue) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(column),
        right: Box::new(Expression::Literal(Literal::Date(date))),
    }
}

/// `left AND right`.
pub(crate) fn and(left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator: BinaryOperator::And,
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// The day after `date`; `None` after 9999-12-31.
fn next_day(date: DateValue) -> Option<DateValue> {
    let (year, month, day) = (date.year(), date.month(), date.day());
    DateValue::new(year, month, day + 1)
        .or_else(|| DateValue::new(year, month + 1, 1))
        .or_else(|| DateValue::new(year + 1, 1, 1))
}
