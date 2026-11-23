//! The metrics of a generated model: base, derived, semi-additive and context (R5).

use crate::model_shape::ModelShape;

/// One generated metric: its name, ANSI SQL expression and optional Menura extension (JSON).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Metric {
    pub(crate) name: String,
    pub(crate) expression: String,
    pub(crate) extension: Option<String>,
}

const AGGREGATES: [&str; 5] = ["SUM", "COUNT", "AVG", "MIN", "MAX"];
const MEASURES: [&str; 3] = ["amount", "quantity", "cost"];

/// The name of the base metric at `index` (counted from 0).
pub(crate) fn base_name(index: usize) -> String {
    format!("base_metric_{:03}", index + 1)
}

/// The name of the derived metric at `index` (counted from 0).
pub(crate) fn derived_name(index: usize) -> String {
    format!("derived_metric_{:03}", index + 1)
}

/// The name of the semi-additive metric at `index` (counted from 0).
pub(crate) fn semi_additive_name(index: usize) -> String {
    format!("stock_metric_{:03}", index + 1)
}

/// The name of the context metric at `index` (counted from 0).
pub(crate) fn context_name(index: usize) -> String {
    format!("context_metric_{:03}", index + 1)
}

fn extension(member: &str) -> String {
    format!(r#"{{"namespace": "menura", "version": 1, {member}}}"#)
}

/// Every metric of the model of `shape`: base, derived, semi-additive, then context metrics.
pub(crate) fn metrics(shape: &ModelShape) -> Vec<Metric> {
    let mut all: Vec<Metric> = (0..shape.base_metrics).map(base).collect();
    all.extend((0..shape.derived_metrics).map(|index| derived(index, shape.base_metrics)));
    all.extend((0..shape.semi_additive_metrics).map(semi_additive));
    all.extend((0..shape.context_metrics).map(|index| context(index, shape.base_metrics)));
    all
}

fn base(index: usize) -> Metric {
    let aggregate = AGGREGATES[index % AGGREGATES.len()];
    let measure = MEASURES[(index / AGGREGATES.len()) % MEASURES.len()];
    Metric {
        name: base_name(index),
        expression: format!("{aggregate}(sales.{measure})"),
        extension: None,
    }
}

fn derived(index: usize, base_metrics: usize) -> Metric {
    let left = base_name((2 * index) % base_metrics);
    let right = base_name((2 * index + 1) % base_metrics);
    let (operator, member) = if index.is_multiple_of(2) {
        ("/", r#""derived": {"on_division_by_zero": "null"}"#)
    } else {
        ("+", r#""derived": {}"#)
    };
    Metric {
        name: derived_name(index),
        expression: format!("AGG({left}) {operator} AGG({right})"),
        extension: Some(extension(member)),
    }
}

fn semi_additive(index: usize) -> Metric {
    let aggregation = if index.is_multiple_of(2) {
        "max"
    } else {
        "min"
    };
    Metric {
        name: semi_additive_name(index),
        expression: "SUM(stock.quantity)".to_owned(),
        extension: Some(extension(&format!(
            r#""additivity": {{"type": "semi_additive", "over": "stock.warehouse", "entity": ["stock.product"], "aggregation": "{aggregation}", "nulls": "keep"}}"#
        ))),
    }
}

/// A context metric over a `SUM` base metric: a prior period of `sales.sale_date` for an
/// even index, a share of the total without `star_001.star_001_attribute_01` for an odd one.
fn context(index: usize, base_metrics: usize) -> Metric {
    let base = base_name((AGGREGATES.len() * index) % base_metrics);
    let modifier = if index.is_multiple_of(2) {
        format!(
            r#"{{"type": "prior_period", "time_dimension": "sales.sale_date", "grain": "month", "offset": {}}}"#,
            index / 2 % 12 + 1
        )
    } else {
        r#"{"type": "share_of_total", "remove": ["star_001.star_001_attribute_01"]}"#.to_owned()
    };
    Metric {
        name: context_name(index),
        expression: format!("AGG({base})"),
        extension: Some(extension(&format!(
            r#""context": {{"base": "{base}", "modifier": {modifier}}}"#
        ))),
    }
}
