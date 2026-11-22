//! The query-filter predicate: the query filters as one conjunction, followed by the
//! `date_range` predicate (E4-4 step 4, QRY-4.2), or by the shifted range of a
//! `prior_period` context query (E4-8.4).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{BinaryOperator, DateValue, Expression, Literal, Reference};
use menura_model::internal_model::{FieldPath, ValueType};
use menura_representations::bound_query::{BoundDateRange, BoundFilter, BoundQuery};
use menura_representations::logical_plan::{Operation, PeriodShift, PlanNode, ShiftedRange};
use menura_representations::origin::{Origin, QueryLocation};
use menura_representations::paths::QueryElement;

use crate::columns::{column_reference, field_name};
use crate::dimensions::at;
use crate::error::{InvariantViolation, PlanError};
use crate::layout::Layout;
use crate::nodes::filter;
use crate::rewrite::rewrite;

/// The shift of the `date_range` of a `prior_period` context query (E4-8.4): a `date_range`
/// on `field` is replaced by its shift.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RangeShift<'query> {
    /// The modifier's `time_dimension`.
    pub(crate) field: &'query FieldPath,
    /// The shift of its query-grain periods.
    pub(crate) shift: PeriodShift,
}

/// The query filter of a branch: its predicate, the shifted range of a `prior_period` context
/// query, and the element it comes from.
#[derive(Debug, Clone)]
pub(crate) struct QueryPredicate {
    /// The query filters and the `date_range`, as one conjunction; `None` when the only term
    /// is the shifted range.
    predicate: Option<Expression>,
    /// The shifted `date_range` (E4-8.4).
    shifted: Option<ShiftedRange>,
    /// The first filter, or the date range.
    origin: QueryLocation,
}

impl QueryPredicate {
    /// `input` filtered: a `Filter`, or a `ShiftedFilter` with the shifted range, whose origin
    /// is the query element, so that the query's values are bound parameters (E6-4.2).
    pub(crate) fn filter(self, input: PlanNode) -> PlanNode {
        let origin = Origin::Query(self.origin);
        match (self.predicate, self.shifted) {
            (predicate, Some(range)) => {
                let schema = input.schema().clone();
                PlanNode::new(
                    Operation::ShiftedFilter {
                        input: Box::new(input),
                        predicate,
                        range,
                    },
                    schema,
                    origin,
                )
            }
            (Some(predicate), None) => filter(input, predicate, origin),
            (None, None) => input,
        }
    }

    /// The expressions it reads.
    pub(crate) fn expressions(&self) -> impl Iterator<Item = &Expression> {
        self.predicate
            .iter()
            .chain(self.shifted.iter().map(|range| &range.column))
    }
}

/// The predicate of the query-filter `Filter` over the columns of the `Join` of the branch,
/// each field read through the join instance its element is bound to (E3-4.2a), with the
/// element it comes from: the first filter, or the date range. `None` without filters and
/// date range.
///
/// # Errors
///
/// An internal error for a filter or date range that reads a field outside its instance, or
/// a date range the plan cannot write.
pub(crate) fn query_predicate(
    query: &BoundQuery,
    layout: &Layout<'_>,
    range_shift: Option<RangeShift<'_>>,
) -> Result<Option<QueryPredicate>, PlanError> {
    predicate_of(query, query.date_range(), layout, range_shift)
}

/// [`query_predicate`] without the `date_range` predicate when its field is `over`: the read
/// that finds carried values keeps the row policies and the other query filters (E4-7.5).
///
/// # Errors
///
/// As [`query_predicate`].
pub(crate) fn query_predicate_outside_range(
    query: &BoundQuery,
    over: &FieldPath,
    layout: &Layout<'_>,
    range_shift: Option<RangeShift<'_>>,
) -> Result<Option<QueryPredicate>, PlanError> {
    let range = query.date_range().filter(|range| &range.field != over);
    predicate_of(query, range, layout, range_shift)
}

fn predicate_of(
    query: &BoundQuery,
    date_range: Option<&BoundDateRange>,
    layout: &Layout<'_>,
    range_shift: Option<RangeShift<'_>>,
) -> Result<Option<QueryPredicate>, PlanError> {
    let mut terms = Vec::new();
    let mut shifted = None;
    for (position, filter) in query.filters().iter().enumerate() {
        terms.push(
            filter_term(filter, position, layout)
                .map_err(|violation| at(violation, &filter.origin))?,
        );
    }
    if let Some(range) = date_range {
        let shift = range_shift
            .filter(|range_shift| range_shift.field == &range.field)
            .map(|range_shift| range_shift.shift);
        let in_range = |violation| at(violation, &range.origin);
        match shift {
            Some(shift) => shifted = Some(shifted_range(range, layout, shift).map_err(in_range)?),
            None => terms.push(date_range_term(range, layout).map_err(in_range)?),
        }
    }
    let origin = query
        .filters()
        .first()
        .map(|filter| &filter.origin)
        .or_else(|| date_range.map(|range| &range.origin));
    let predicate = terms.into_iter().reduce(and);
    if predicate.is_none() && shifted.is_none() {
        return Ok(None);
    }
    let origin = origin.cloned().ok_or_else(|| {
        PlanError::invariant(
            InvariantViolation::UnplannableDateRange,
            &crate::slice::query_root(),
        )
    })?;
    Ok(Some(QueryPredicate {
        predicate,
        shifted,
        origin,
    }))
}

/// The filter at `position`, with every reference turned into the reference to the column
/// of its field, read through the instance the filter is bound to for that field's dataset.
/// An unqualified reference reads the filter's only dataset.
fn filter_term(
    filter: &BoundFilter,
    position: usize,
    layout: &Layout<'_>,
) -> Result<Expression, InvariantViolation> {
    rewrite(&filter.expression, &mut |node| {
        let Expression::Reference(reference) = node else {
            return Ok(None);
        };
        let field = filter_field(filter, reference)?;
        let element = QueryElement::Filter {
            filter: position,
            dataset: field.dataset.clone(),
        };
        let column = layout.element_column(&element, &field)?;
        column_reference(column.as_str()).map(Some)
    })
}

/// The field `reference` names in `filter`: in the dataset its qualifier names, or in the
/// filter's only dataset when it has no qualifier.
fn filter_field(
    filter: &BoundFilter,
    reference: &Reference,
) -> Result<FieldPath, InvariantViolation> {
    let dataset = match (&reference.qualifier, filter.datasets.as_slice()) {
        (Some(qualifier), _) => field_name(qualifier),
        (None, [only]) => only.clone(),
        (None, _) => return Err(InvariantViolation::ForeignField),
    };
    Ok(FieldPath {
        dataset,
        field: field_name(&reference.name),
    })
}

/// `x >= from AND x <= to` for a date field; `x >= from AND x < to + 1 day` for a timestamp
/// field, with `date` literals that E6 converts to the start of that day (E6-4.2b). When `to`
/// is 9999-12-31, a timestamp field has no upper bound (QRY-4.2).
fn date_range_term(
    range: &BoundDateRange,
    layout: &Layout<'_>,
) -> Result<Expression, InvariantViolation> {
    let element = QueryElement::DateRange;
    let column = column_reference(layout.element_column(&element, &range.field)?.as_str())?;
    let field_type = layout
        .dataset(layout.bound(&element))?
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

/// The shifted range of E4-8.4 on the column of the `date_range` field, which E6 writes from
/// the query's `from` and `to` (E6-4.14).
fn shifted_range(
    range: &BoundDateRange,
    layout: &Layout<'_>,
    shift: PeriodShift,
) -> Result<ShiftedRange, InvariantViolation> {
    let element = QueryElement::DateRange;
    let column = column_reference(layout.element_column(&element, &range.field)?.as_str())?;
    let field_type = layout
        .dataset(layout.bound(&element))?
        .field(&range.field.field)
        .and_then(|field| field.typed.as_ref())
        .map(|typed| &typed.value_type);
    if !matches!(
        field_type,
        Some(ValueType::Logical(
            LogicalType::Date | LogicalType::Timestamp { .. }
        ))
    ) {
        return Err(InvariantViolation::UnplannableDateRange);
    }
    Ok(ShiftedRange {
        column,
        field: range.field.clone(),
        from: range.from,
        to: range.to,
        shift,
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
