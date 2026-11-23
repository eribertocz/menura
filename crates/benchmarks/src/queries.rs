//! The reference queries, the sweep queries and the security context of the benchmarks (R5,
//! R6).

use crate::datasets::{attribute_name, chain_name, star_name};
use crate::metrics::{base_name, context_name, derived_name, semi_additive_name};
use crate::model_shape::ModelShape;

/// One generated query document and the name the benchmark reports it under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkQuery {
    /// The name of the query in reports and in `benchmarks/baseline.json`.
    pub name: String,
    /// The query document, as bytes.
    pub document: Vec<u8>,
}

/// The security context every benchmark query is compiled with: role `analyst` and two
/// regions, so the row policies of the model apply (R5).
#[must_use]
pub fn security_context_document() -> Vec<u8> {
    br#"{"contract": "menura.security_context", "contract_version": "1.1.0", "subject": "benchmark-analyst", "roles": ["analyst"], "attributes": {"regions": ["region_1", "region_2"]}}"#.to_vec()
}

fn wide_table(dimensions: &[String], metrics: &[String], members: &str) -> Vec<u8> {
    let dimensions: Vec<String> = dimensions
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect();
    let metrics: Vec<String> = metrics
        .iter()
        .map(|name| format!(r#"{{"metric": "{name}", "alias": "{name}"}}"#))
        .collect();
    format!(
        r#"{{"contract": "menura.query", "contract_version": "1.1.0", "time_zone": "UTC", "query": {{"dimensions": [{}], "metrics": [{}]{members}}}}}"#,
        dimensions.join(", "),
        metrics.join(", ")
    )
    .into_bytes()
}

fn sql(shape: &ModelShape, text: &str) -> Vec<u8> {
    format!(
        r#"{{"contract": "menura.query", "contract_version": "1.1.0", "time_zone": "UTC", "sql": "{}"}}"#,
        text.replace("{model}", shape.name)
    )
    .into_bytes()
}

fn compiler_extension(members: &str) -> String {
    format!(
        r#", "nonportable_extensions": [{{"vendor_name": "COMMON", "data": {{"namespace": "menura", "version": 1, {members}}}}}]"#
    )
}

fn star(index: usize, attribute: usize) -> String {
    let dataset = star_name(index);
    format!("{dataset}.{}", attribute_name(&dataset, attribute))
}

fn chain(index: usize) -> String {
    let dataset = chain_name(index);
    format!("{dataset}.{}", attribute_name(&dataset, 1))
}

fn query(name: &str, document: Vec<u8>) -> BenchmarkQuery {
    BenchmarkQuery {
        name: name.to_owned(),
        document,
    }
}

fn names(name: fn(usize) -> String, indexes: &[usize]) -> Vec<String> {
    indexes.iter().map(|index| name(*index)).collect()
}

/// The 20 reference queries of R5 on the model of `shape`: dimension-only, base, derived,
/// semi-additive and context metrics, filters, time grains, date ranges, order and limit,
/// the chain, propagated row policies and the SQL form, which together cover the slice of
/// VS-1f.
#[must_use]
pub fn reference_queries(shape: &ModelShape) -> Vec<BenchmarkQuery> {
    let mut all = base_and_derived_queries();
    all.extend(additivity_and_context_queries());
    all.extend(modifier_queries());
    all.extend(structure_queries(shape));
    all
}

fn base_and_derived_queries() -> Vec<BenchmarkQuery> {
    let none: [String; 0] = [];
    vec![
        query("q01_dimension_only", wide_table(&[star(11, 1)], &none, "")),
        query("q02_one_metric", wide_table(&none, &[base_name(0)], "")),
        query(
            "q03_propagated_policy",
            wide_table(&[star(1, 2)], &[base_name(0)], ""),
        ),
        query(
            "q04_star_dimensions",
            wide_table(
                &[star(1, 1), star(12, 1), star(20, 2)],
                &names(base_name, &[0, 1, 2, 3]),
                "",
            ),
        ),
        query(
            "q05_derived",
            wide_table(&[star(12, 1)], &[derived_name(0)], ""),
        ),
        query(
            "q06_derived_and_base",
            wide_table(
                &[star(13, 1)],
                &[derived_name(0), derived_name(1), base_name(5)],
                "",
            ),
        ),
    ]
}

fn additivity_and_context_queries() -> Vec<BenchmarkQuery> {
    let none: [String; 0] = [];
    vec![
        query(
            "q07_semi_additive",
            wide_table(&["stock.product".to_owned()], &[semi_additive_name(0)], ""),
        ),
        query(
            "q08_semi_additive_total",
            wide_table(&none, &names(semi_additive_name, &[0, 1]), ""),
        ),
        query(
            "q09_prior_period",
            wide_table(
                &["sales.sale_date".to_owned()],
                &[context_name(0), base_name(0)],
                &compiler_extension(
                    r#""time_grains": [{"dimension": "sales.sale_date", "grain": "month"}]"#,
                ),
            ),
        ),
        query(
            "q10_share_of_total",
            wide_table(&[star(1, 1)], &[context_name(1)], ""),
        ),
    ]
}

fn modifier_queries() -> Vec<BenchmarkQuery> {
    vec![
        query(
            "q11_filter",
            wide_table(
                &[star(2, 1)],
                &[base_name(0)],
                r#", "filter": "sales.quantity >= 1""#,
            ),
        ),
        query(
            "q12_date_range",
            wide_table(
                &[star(14, 1)],
                &[base_name(1)],
                &compiler_extension(
                    r#""date_range": {"dimension": "sales.sale_date", "from": "2026-01-01", "to": "2026-06-30"}"#,
                ),
            ),
        ),
        query(
            "q13_order_and_limit",
            wide_table(
                &[star(15, 1)],
                &[base_name(0)],
                &compiler_extension(
                    r#""order_by": [{"output": "base_metric_001", "direction": "desc", "nulls": "last"}, {"output": "star_015_attribute_01", "direction": "asc", "nulls": "last"}], "limit": 10"#,
                ),
            ),
        ),
        query(
            "q14_time_grain_year",
            wide_table(
                &["sales.sale_date".to_owned()],
                &names(base_name, &[0, 1, 2]),
                &compiler_extension(
                    r#""time_grains": [{"dimension": "sales.sale_date", "grain": "year"}]"#,
                ),
            ),
        ),
        query(
            "q15_summary_candidate",
            wide_table(
                &["sales.sale_date".to_owned()],
                &[base_name(0)],
                &compiler_extension(
                    r#""time_grains": [{"dimension": "sales.sale_date", "grain": "month"}]"#,
                ),
            ),
        ),
    ]
}

fn structure_queries(shape: &ModelShape) -> Vec<BenchmarkQuery> {
    let last_chain = chain(shape.chain_datasets);
    let middle_chain = chain(shape.chain_datasets / 2);
    let wide_dimensions: Vec<String> = (11..19).map(|index| star(index, 1)).collect();
    let mut wide_metrics = names(base_name, &[0, 1, 2, 3, 4, 5]);
    wide_metrics.extend(names(derived_name, &[2, 3]));
    vec![
        query(
            "q16_chain_end",
            wide_table(&[last_chain], &[base_name(0)], ""),
        ),
        query(
            "q17_chain_and_star",
            wide_table(&[middle_chain, star(3, 1)], &[base_name(2)], ""),
        ),
        query("q18_wide", wide_table(&wide_dimensions, &wide_metrics, "")),
        query(
            "q19_sql_form",
            sql(
                shape,
                "SELECT DATE_TRUNC('month', sale_date) AS sale_month, AGG(base_metric_001) AS total FROM {model}.sales GROUP BY ALL ORDER BY sale_month",
            ),
        ),
        query(
            "q20_sql_form_filter",
            sql(
                shape,
                "SELECT DATE_TRUNC('year', sale_date) AS sale_year, AGG(base_metric_002) AS total FROM {model}.sales WHERE quantity >= 1 GROUP BY ALL",
            ),
        ),
    ]
}

/// The sweep query of R3 with `items` dimensions and `items` metrics. The joined datasets
/// stay the same for every item count: every dimension is an attribute of the last star
/// dataset, and every metric has `sales` as its home dataset (base metrics, then derived
/// metrics). Only the number of items grows.
#[must_use]
pub fn sweep_query(shape: &ModelShape, items: usize) -> BenchmarkQuery {
    let dimensions: Vec<String> = (1..=items)
        .map(|attribute| star(shape.star_datasets, attribute))
        .collect();
    let metrics: Vec<String> = (0..items)
        .map(|index| {
            if index < shape.base_metrics {
                base_name(index)
            } else {
                derived_name(index - shape.base_metrics)
            }
        })
        .collect();
    query(
        &format!("sweep_{items:02}"),
        wide_table(&dimensions, &metrics, ""),
    )
}

/// The item counts of the R3 sweep: 1, 2, 4, …, 64.
pub const SWEEP_ITEMS: [usize; 7] = [1, 2, 4, 8, 16, 32, 64];
