//! Query trees, as E1 would build them, and the admitted security contexts of the tests.

use menura_contracts::types::common::Grain;
use menura_contracts::types::query::DateBounds;
use menura_model::expression::{
    AggregateCall, BinaryOperator, Expression, Identifier, Literal, Reference,
};
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;
use menura_representations::query_tree::{
    QueryDateRange, QueryDimension, QueryDimensionKind, QueryFilter, QueryMetric, QueryMetricKind,
    QueryPath, QueryReference, QueryTree, QueryTreeParts, ReferencePart, TimeGrainChoice,
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
        kind: QueryDimensionKind::Field {
            reference,
            time_grain: None,
        },
        origin: at(&format!("/query/dimensions/{index}")),
    }
}

/// The dimension `text` with a time grain.
pub fn dimension_with_grain(index: usize, text: &str, grain: Grain) -> QueryDimension {
    let mut dimension = dimension(index, text);
    if let QueryDimensionKind::Field { time_grain, .. } = &mut dimension.kind {
        *time_grain = Some(TimeGrainChoice {
            grain,
            origin: at("/query/nonportable_extensions/0/data/time_grains/0"),
        });
    }
    dimension
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
