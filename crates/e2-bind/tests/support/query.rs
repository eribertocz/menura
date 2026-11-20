//! Query trees, as E1 would build them, and the admitted security contexts of the tests.

use menura_contracts::types::common::Grain;
use menura_contracts::types::query::{DateBounds, Direction, NullsOrder, RelativeRange};
use menura_model::expression::{
    AggregateCall, BinaryOperator, Expression, Identifier, Literal, Reference,
};
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;
use menura_representations::query_tree::{
    QueryDateRange, QueryDimension, QueryDimensionKind, QueryFilter, QueryLimit, QueryMetric,
    QueryMetricKind, QueryOrderItem, QueryPath, QueryReference, QueryTree, QueryTreeParts,
    ReferencePart, TimeGrainChoice,
};
use menura_representations::time::{QueryTime, TimeZoneName};
use menura_security::{AdmittedSecurityContext, admit_security_context};

use super::model::declarations;

/// The location at `pointer`.
pub fn at(pointer: &str) -> QueryLocation {
    QueryLocation::new(pointer, None)
}

/// A reference written as `text`, split at each `.`; each part is regular.
pub fn reference(text: &str) -> QueryReference {
    parts(
        text.split('.')
            .map(|part| ReferencePart::Regular(part.to_owned()))
            .collect(),
    )
}

/// A reference with exactly `parts`.
pub fn parts(parts: Vec<ReferencePart>) -> QueryReference {
    QueryReference::new(parts).expect("at least one part")
}

/// The dimension `text` at `/query/dimensions/<index>`.
pub fn dimension(index: usize, text: &str) -> QueryDimension {
    dimension_reference(index, reference(text))
}

/// The dimension `reference` at `/query/dimensions/<index>`.
pub fn dimension_reference(index: usize, reference: QueryReference) -> QueryDimension {
    QueryDimension {
        output_name: OutputName::from_reference(&reference),
        kind: QueryDimensionKind::Field { reference },
        origin: at(&format!("/query/dimensions/{index}")),
    }
}

/// The `time_grains` entry `grain` on `text`, at `pointer`.
pub fn time_grain(pointer: &str, text: &str, grain: Grain) -> TimeGrainChoice {
    TimeGrainChoice {
        dimension: reference(text),
        grain,
        origin: at(pointer),
    }
}

/// The metric `text` at `/query/metrics/<index>`.
pub fn metric(index: usize, text: &str) -> QueryMetric {
    let reference = reference(text);
    QueryMetric {
        output_name: OutputName::from_reference(&reference),
        kind: QueryMetricKind::Reference(reference),
        origin: at(&format!("/query/metrics/{index}")),
    }
}

/// The expression reference `qualifier.name`, or `name` without a qualifier.
pub fn column(text: &str) -> Expression {
    let mut names = text.rsplitn(2, '.');
    let name = names.next().expect("a name");
    Expression::Reference(Reference {
        qualifier: names
            .next()
            .map(|qualifier| Identifier::regular(qualifier).expect("an identifier")),
        name: Identifier::regular(name).expect("an identifier"),
    })
}

/// `AGG(qualifier.metric)`, or `AGG(metric)` without a qualifier.
pub fn metric_aggregate(text: &str) -> Expression {
    match column(text) {
        Expression::Reference(reference) => Expression::MetricAggregate(reference),
        other => other,
    }
}

/// `left = 'x'`.
pub fn equals_text(left: Expression) -> Expression {
    Expression::Binary {
        operator: BinaryOperator::Equal,
        left: Box::new(left),
        right: Box::new(Expression::Literal(Literal::Text("x".to_owned()))),
    }
}

/// `COUNT(*) > TRUE`: a filter that holds an aggregate.
pub fn count_rows_filter() -> Expression {
    Expression::Binary {
        operator: BinaryOperator::Greater,
        left: Box::new(Expression::Aggregate {
            call: AggregateCall::CountRows,
            filter: None,
        }),
        right: Box::new(Expression::Literal(Literal::Boolean(true))),
    }
}

/// The filter `expression` at `pointer`.
pub fn filter(pointer: &str, expression: Expression) -> QueryFilter {
    QueryFilter {
        expression,
        origin: at(pointer),
    }
}

/// The date range on `text` at `pointer`.
pub fn date_range(pointer: &str, text: &str) -> QueryDateRange {
    QueryDateRange {
        dimension: reference(text),
        bounds: DateBounds::Absolute {
            from: "2026-01-01".to_owned(),
            to: "2026-03-31".to_owned(),
        },
        origin: at(pointer),
    }
}

/// The path from `from` to `to` through `relationships`, at `pointer`.
pub fn path(pointer: &str, from: &str, to: &str, relationships: &[&str]) -> QueryPath {
    QueryPath::new(
        reference(from),
        reference(to),
        relationships.iter().map(|text| reference(text)).collect(),
        at(pointer),
    )
    .expect("one relationship")
}

/// The time zone `UTC` without a reference time.
pub fn utc() -> QueryTime {
    QueryTime {
        time_zone: TimeZoneName::new("UTC"),
        reference_time: None,
        reference_date: None,
    }
}

/// The wide-table tree of `dimensions`, `metrics` and `parts`.
pub fn tree(
    dimensions: Vec<QueryDimension>,
    metrics: Vec<QueryMetric>,
    parts: QueryTreeParts,
) -> QueryTree {
    QueryTree::wide_table(dimensions, metrics, utc(), parts)
}

/// The admitted context of a user holding `held` roles.
pub fn user(held: &[&str]) -> AdmittedSecurityContext {
    let roles: Vec<String> = held.iter().map(|role| format!("\"{role}\"")).collect();
    let text = format!(
        r#"{{"contract": "menura.security_context", "contract_version": "1.1.0",
            "subject": "user-1", "roles": [{}], "attributes": {{}}}}"#,
        roles.join(", ")
    );
    admit_security_context(&text, Some(&declarations())).expect("an admitted context")
}

/// The `order_by` item on `output` at `/query/nonportable_extensions/0/data/order_by/<index>`.
pub fn order_item(index: usize, output: &str) -> QueryOrderItem {
    QueryOrderItem {
        output: OutputName::new(output),
        direction: Direction::Asc,
        nulls: NullsOrder::Last,
        origin: at(&format!(
            "/query/nonportable_extensions/0/data/order_by/{index}"
        )),
    }
}

/// The limit of `rows` rows.
pub fn limit(rows: u64) -> QueryLimit {
    QueryLimit {
        rows,
        origin: at("/query/nonportable_extensions/0/data/limit"),
    }
}

/// The relative date range on `text` at `pointer`.
pub fn relative_range(
    pointer: &str,
    text: &str,
    unit: Grain,
    count: u64,
    include_current: Option<bool>,
) -> QueryDateRange {
    QueryDateRange {
        dimension: reference(text),
        bounds: DateBounds::Relative(RelativeRange {
            unit,
            count,
            include_current,
        }),
        origin: at(pointer),
    }
}

/// The absolute date range on `text` from `from` to `to`, at `pointer`.
pub fn absolute_range(pointer: &str, text: &str, from: &str, to: &str) -> QueryDateRange {
    QueryDateRange {
        dimension: reference(text),
        bounds: DateBounds::Absolute {
            from: from.to_owned(),
            to: to.to_owned(),
        },
        origin: at(pointer),
    }
}

/// The wide-table tree of `dimensions`, `metrics` and `parts`, with `time`.
pub fn tree_at(
    dimensions: Vec<QueryDimension>,
    metrics: Vec<QueryMetric>,
    time: QueryTime,
    parts: QueryTreeParts,
) -> QueryTree {
    QueryTree::wide_table(dimensions, metrics, time, parts)
}

/// The date `year-month-day`.
pub fn date(year: u16, month: u8, day: u8) -> menura_model::expression::DateValue {
    menura_model::expression::DateValue::new(year, month, day).expect("a date")
}

/// `Asia/Tokyo` at 2026-12-31T15:30:00Z, whose date there is 2027-01-01, as E1 computes it.
pub fn tokyo_new_year() -> QueryTime {
    use menura_model::expression::{TimeOfDay, TimestampValue};
    use menura_representations::time::{ReferenceTime, UtcOffset};
    QueryTime {
        time_zone: TimeZoneName::new("Asia/Tokyo"),
        reference_time: Some(ReferenceTime {
            local: TimestampValue {
                date: date(2026, 12, 31),
                time: TimeOfDay::new(15, 30, 0, "").expect("a time of day"),
            },
            offset: UtcOffset::from_minutes(0).expect("an offset"),
        }),
        reference_date: Some(date(2027, 1, 1)),
    }
}
