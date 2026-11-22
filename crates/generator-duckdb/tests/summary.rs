//! Summary scans and recomposed components (E6-4.15, SUM-3.8): a `Scan` of a summary reads its
//! `source` like any relation, and each recomposed component has the result type of its base
//! component.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{FloatBits, LogicalType};
use menura_generator_duckdb::{GeneratedSql, InvariantViolation};
use menura_representations::logical_plan::{AggregateComponent, PlanNode, Recomposition};
use menura_representations::names::ColumnName;
use support::{
    aggregate, assert_golden, big_integer, column, component, decimal, generate_summary,
    huge_integer, named, query, summary_scan,
};

fn double() -> LogicalType {
    LogicalType::Float {
        bits: FloatBits::SixtyFour,
    }
}

/// The component `name` computed by `call` over the summary and recomposed as `recomposition`.
fn recomposed(name: &str, call: &str, recomposition: Recomposition) -> AggregateComponent {
    AggregateComponent {
        recomposition: Some(recomposition),
        ..component(name, call, None)
    }
}

/// The `Aggregate` per region over `fields` of `dataset`, computing `metric` of type
/// `result_type`.
fn per_region(
    dataset: &str,
    fields: &[&str],
    metric: AggregateComponent,
    result_type: LogicalType,
) -> PlanNode {
    let name = metric.name.as_str().to_owned();
    aggregate(
        summary_scan(dataset, fields),
        vec![named("REGION", "REGION")],
        vec![metric],
        vec![
            column("REGION", LogicalType::Text, true),
            column(&name, result_type, true),
        ],
    )
}

fn generated(plan: &PlanNode, metric: &str) -> GeneratedSql {
    generate_summary(plan, &query(&["REGION"], &[metric])).expect("SQL")
}

fn physical_type(generated: &GeneratedSql, name: &str) -> String {
    generated
        .result_schema
        .iter()
        .find(|column| column.name == name)
        .map(|column| column.physical_type.clone())
        .expect("a result column")
}

fn summary_sum() -> PlanNode {
    per_region(
        "ORDERS_BY_REGION",
        &["REGION", "QUANTITY_SUM"],
        recomposed("TOTAL_QUANTITY", "SUM(QUANTITY_SUM)", Recomposition::Sum),
        huge_integer(),
    )
}

fn summary_count() -> PlanNode {
    per_region(
        "ORDERS_BY_REGION",
        &["REGION", "QUANTITY_COUNT"],
        recomposed("QUANTITY_COUNT", "SUM(QUANTITY_COUNT)", Recomposition::Count),
        big_integer(),
    )
}

fn summary_maximum() -> PlanNode {
    per_region(
        "ORDERS_BY_REGION",
        &["REGION", "AMOUNT_MAX"],
        recomposed("LARGEST_AMOUNT", "MAX(AMOUNT_MAX)", Recomposition::MinMax),
        decimal(10, 2),
    )
}

fn summary_average() -> PlanNode {
    per_region(
        "ORDERS_BY_REGION",
        &["REGION", "AMOUNT_SUM", "AMOUNT_COUNT"],
        recomposed(
            "AVERAGE_AMOUNT",
            "SUM(AMOUNT_SUM)",
            Recomposition::Average {
                count: ColumnName::new("AMOUNT_COUNT"),
            },
        ),
        double(),
    )
}

#[test]
fn e6_4_15_summary_sum_keeps_its_type() {
    let generated = generated(&summary_sum(), "TOTAL_QUANTITY");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_15_summary_sum.sql"),
    );
    assert_eq!(physical_type(&generated, "TOTAL_QUANTITY"), "HUGEINT");
}

#[test]
fn e6_4_15_summary_count_is_a_coalesced_sum() {
    let generated = generated(&summary_count(), "QUANTITY_COUNT");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_15_summary_count.sql"),
    );
    assert_eq!(physical_type(&generated, "QUANTITY_COUNT"), "BIGINT");
}

#[test]
fn e6_4_15_summary_maximum_is_generated_as_is() {
    let generated = generated(&summary_maximum(), "LARGEST_AMOUNT");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_15_summary_maximum.sql"),
    );
    assert_eq!(physical_type(&generated, "LARGEST_AMOUNT"), "DECIMAL(10,2)");
}

#[test]
fn e6_4_15_summary_average_divides_the_recomposed_sum_by_the_recomposed_count() {
    let generated = generated(&summary_average(), "AVERAGE_AMOUNT");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_15_summary_average.sql"),
    );
    assert_eq!(physical_type(&generated, "AVERAGE_AMOUNT"), "DOUBLE");
}

/// The same metric per region over the base `ORDERS`.
fn base(metric: AggregateComponent, result_type: LogicalType) -> PlanNode {
    per_region("ORDERS", &["REGION", "QUANTITY", "AMOUNT"], metric, result_type)
}

#[test]
fn sum_3_8_recomposed_components_have_the_type_of_their_base() {
    let pairs = [
        (
            summary_sum(),
            base(component("TOTAL_QUANTITY", "SUM(QUANTITY)", None), huge_integer()),
            "TOTAL_QUANTITY",
        ),
        (
            summary_count(),
            base(component("QUANTITY_COUNT", "COUNT(QUANTITY)", None), big_integer()),
            "QUANTITY_COUNT",
        ),
        (
            summary_maximum(),
            base(component("LARGEST_AMOUNT", "MAX(AMOUNT)", None), decimal(10, 2)),
            "LARGEST_AMOUNT",
        ),
        (
            summary_average(),
            base(component("AVERAGE_AMOUNT", "AVG(AMOUNT)", None), double()),
            "AVERAGE_AMOUNT",
        ),
    ];
    for (summary, base, metric) in pairs {
        assert_eq!(
            generated(&summary, metric).result_schema,
            generated(&base, metric).result_schema,
            "{metric}"
        );
    }
}

#[test]
fn e6_4_15_a_count_recomposition_reads_a_sum_of_counts() {
    let plan = per_region(
        "ORDERS_BY_REGION",
        &["REGION", "QUANTITY_COUNT"],
        recomposed("QUANTITY_COUNT", "COUNT(QUANTITY_COUNT)", Recomposition::Count),
        big_integer(),
    );
    let error = generate_summary(&plan, &query(&["REGION"], &["QUANTITY_COUNT"]))
        .expect_err("a malformed recomposition");
    assert_eq!(error.violation, InvariantViolation::MalformedRecomposition);
}
