//! E2-5 row 12b, E2-5.6: `EQRY-03` for a metric or dimension whose model expression reads the
//! current date or time, directly or through the fields and metrics it reads (QRY-2,
//! E0F-2.3).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_e2_bind::BindCode;
use menura_model::expression::{Expression, Identifier, Reference};
use menura_model::internal_model::{InternalModel, ModelParts};
use menura_representations::query_tree::QueryTreeParts;

use support::model::{model_with, name, semantic_metric};
use support::query::{dimension, filter, metric, tokyo_new_year, tree, tree_at};
use support::{bind_error, bind_query};

const ANALYST: &[&str] = &["analyst"];

fn error(code: BindCode, pointer: &str) -> (BindCode, String) {
    (code, pointer.to_owned())
}

/// `CURRENT_DATE IS NOT NULL`.
fn today_is_known() -> Expression {
    Expression::IsNull {
        operand: Box::new(Expression::CurrentDate),
        negated: true,
    }
}

/// `CASE WHEN CURRENT_DATE IS NOT NULL THEN value END`.
fn when_today(value: Expression) -> Expression {
    Expression::Case {
        operand: None,
        branches: vec![menura_model::expression::CaseBranch {
            condition: today_is_known(),
            result: value,
        }],
        otherwise: None,
    }
}

/// The field `field` of `ORDERS`.
fn orders_field<'parts>(
    parts: &'parts mut ModelParts,
    field: &str,
) -> &'parts mut menura_model::internal_model::Field {
    parts
        .datasets
        .iter_mut()
        .find(|dataset| dataset.name == name("ORDERS"))
        .and_then(|dataset| {
            dataset
                .fields
                .iter_mut()
                .find(|found| found.name == name(field))
        })
        .expect("a field of ORDERS")
}

/// The model whose field `ORDERS.field` reads the current date.
fn field_reads_today(field: &str) -> InternalModel {
    model_with(|parts| {
        let found = orders_field(parts, field);
        let column = found.expression.take().expect("an expression");
        found.expression = Some(when_today(column));
    })
}

/// The model whose metric `REVENUE` reads the current date in its own expression.
fn revenue_reads_today() -> InternalModel {
    model_with(|parts| {
        let revenue = semantic_metric(parts, "REVENUE");
        let Some(Expression::Aggregate { filter, .. }) = revenue.expression.as_mut() else {
            panic!("REVENUE is an aggregate");
        };
        *filter = Some(Box::new(today_is_known()));
    })
}

#[test]
fn e2_5_6_a_metric_whose_expression_reads_the_current_date_is_eqry_03() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        bind_error(&query, &revenue_reads_today(), ANALYST),
        error(BindCode::MissingReferenceTime, "/query/metrics/0")
    );
}

#[test]
fn e2_5_6_a_dimension_whose_field_reads_the_current_date_is_eqry_03() {
    let query = tree(
        vec![
            dimension(0, "orders.customer_id"),
            dimension(1, "orders.region"),
        ],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        bind_error(&query, &field_reads_today("REGION"), ANALYST),
        error(BindCode::MissingReferenceTime, "/query/dimensions/1")
    );
}

#[test]
fn e2_5_6_a_metric_reading_a_field_that_reads_the_current_date_is_eqry_03() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        bind_error(&query, &field_reads_today("AMOUNT"), ANALYST),
        error(BindCode::MissingReferenceTime, "/query/metrics/0")
    );
}

#[test]
fn e2_5_6_a_dataset_metric_reading_a_bare_field_that_reads_the_current_date_is_eqry_03() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "orders.order_total")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        bind_error(&query, &field_reads_today("AMOUNT"), ANALYST),
        error(BindCode::MissingReferenceTime, "/query/metrics/0")
    );
}

#[test]
fn e2_5_6_a_metric_reading_a_metric_that_reads_the_current_date_is_eqry_03() {
    let model = model_with(|parts| {
        let Some(Expression::Aggregate { filter, .. }) =
            semantic_metric(parts, "REVENUE").expression.as_mut()
        else {
            panic!("REVENUE is an aggregate");
        };
        *filter = Some(Box::new(today_is_known()));
        semantic_metric(parts, "COST").expression = Some(Expression::MetricAggregate(Reference {
            qualifier: None,
            name: Identifier::regular("revenue").expect("an identifier"),
        }));
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        bind_error(&query, &model, ANALYST),
        error(BindCode::MissingReferenceTime, "/query/metrics/0")
    );
}

#[test]
fn e2_5_6_a_dimension_is_reported_before_a_metric_in_query_order() {
    let model = model_with(|parts| {
        let region = orders_field(parts, "REGION");
        let column = region.expression.take().expect("an expression");
        region.expression = Some(when_today(column));
        let amount = orders_field(parts, "AMOUNT");
        let column = amount.expression.take().expect("an expression");
        amount.expression = Some(when_today(column));
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        bind_error(&query, &model, ANALYST),
        error(BindCode::MissingReferenceTime, "/query/dimensions/0")
    );
}

#[test]
fn e2_5_6_a_metric_is_reported_before_a_filter_in_query_order() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts {
            filters: vec![filter("/query/filter", today_is_known())],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &revenue_reads_today(), ANALYST),
        error(BindCode::MissingReferenceTime, "/query/metrics/0")
    );
}

#[test]
fn e2_5_6_a_metric_reading_the_current_date_binds_with_reference_time() {
    let query = tree_at(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        tokyo_new_year(),
        QueryTreeParts::default(),
    );
    assert!(bind_query(&query, &revenue_reads_today(), ANALYST).is_ok());
}

#[test]
fn e2_5_6_elements_that_never_read_the_current_date_bind_without_reference_time() {
    let query = tree(
        vec![dimension(0, "orders.customer_id")],
        vec![metric(0, "orders.order_total")],
        QueryTreeParts::default(),
    );
    assert!(bind_query(&query, &field_reads_today("REGION"), ANALYST).is_ok());
}
