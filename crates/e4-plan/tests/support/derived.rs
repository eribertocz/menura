//! The two-facts model of the stitching tests with derived metrics over its metrics, and the
//! plans of their queries (DER-2, DER-4).

use menura_contracts::types::common::LogicalType;
use menura_e4_plan::PlanError;
use menura_expressions::typing::double;
use menura_model::internal_model::{
    Additivity, DerivedMetric, DivisionByZero, InternalModel, Metric, MetricClassification,
    TypedValue, ValueType,
};
use menura_representations::bound_query::{BoundDimension, BoundMetric, BoundQueryParts};
use menura_representations::logical_plan::PlanNode;
use menura_representations::paths::{Branch, Branches};

use super::stitch::two_facts_model;
use super::{anyone, build_plan, located, metric_reference, name, query, tree};

/// The 64-bit float of a division (E0F-5a).
pub fn float() -> LogicalType {
    double()
}

/// The derived metric `metric_name` with the formula `formula`, which reads `inputs`, over
/// the home datasets `homes`, typed as `logical_type`, classified as E0.g does (DER-5.1.6).
fn derived(
    metric_name: &str,
    formula: &str,
    inputs: &[&str],
    homes: &[&str],
    logical_type: LogicalType,
) -> Metric {
    let inputs: Vec<_> = inputs.iter().map(|input| name(input)).collect();
    Metric {
        name: name(metric_name),
        expression: Some(tree(formula)),
        typed: Some(TypedValue {
            value_type: ValueType::Logical(logical_type),
            nullable: true,
        }),
        classification: Some(MetricClassification {
            components: Vec::new(),
            inputs: inputs.clone(),
            home_datasets: homes.iter().map(|home| name(home)).collect(),
            additivity: Additivity::NonAdditive,
            recomposable: true,
        }),
        declared_additivity: None,
        derived: Some(DerivedMetric {
            inputs,
            on_division_by_zero: Some(DivisionByZero::Null),
        }),
        context: None,
        descriptions: menura_model::internal_model::Descriptions::default(),
        query_errors: Vec::new(),
        location: located(&format!("/semantic_model/0/metrics/{metric_name}")),
    }
}

/// [`two_facts_model`] with the derived metrics:
///
/// - `AVERAGE_BUDGET`: `AGG(BUDGET_TOTAL) / AGG(BUDGET_COUNT)`, both of `BUDGETS`;
/// - `SALES_TO_BUDGET`: `AGG(SALES_TOTAL) / AGG(BUDGET_TOTAL)`, across `ORDERS` and `BUDGETS`;
/// - `SALES_TO_BUDGET_PERCENT`: `AGG(SALES_TO_BUDGET) * 100`, a derived metric over a derived
///   metric;
/// - `GAP_SHARE`: `AGG(GAP) / AGG(BUDGET_TOTAL)`, whose first input has two components.
pub fn derived_model() -> InternalModel {
    let base = two_facts_model();
    let mut parts = base.parts().clone();
    parts.metrics.extend([
        derived(
            "AVERAGE_BUDGET",
            "AGG(budget_total) / AGG(budget_count)",
            &["BUDGET_TOTAL", "BUDGET_COUNT"],
            &["BUDGETS"],
            float(),
        ),
        derived(
            "GAP_SHARE",
            "AGG(gap) / AGG(budget_total)",
            &["GAP", "BUDGET_TOTAL"],
            &["BUDGETS", "ORDERS"],
            float(),
        ),
        derived(
            "SALES_TO_BUDGET",
            "AGG(sales_total) / AGG(budget_total)",
            &["SALES_TOTAL", "BUDGET_TOTAL"],
            &["BUDGETS", "ORDERS"],
            float(),
        ),
        derived(
            "SALES_TO_BUDGET_PERCENT",
            "AGG(sales_to_budget) * 100",
            &["SALES_TO_BUDGET"],
            &["BUDGETS", "ORDERS"],
            float(),
        ),
    ]);
    parts
        .metrics
        .sort_by(|left, right| left.name.cmp(&right.name));
    InternalModel::new(parts, base.hash().clone())
}

/// The plan over [`derived_model`] of `dimensions`, the metrics `metric_names` and
/// `branches`.
pub fn derived_plan(
    branches: Vec<Branch>,
    dimensions: Vec<BoundDimension>,
    metric_names: &[&str],
) -> PlanNode {
    plan_over(&derived_model(), branches, dimensions, metric_names).expect("a plan")
}

/// The plan over `model` of `dimensions`, the metrics `metric_names` and `branches`.
pub fn plan_over(
    model: &InternalModel,
    branches: Vec<Branch>,
    dimensions: Vec<BoundDimension>,
    metric_names: &[&str],
) -> Result<PlanNode, PlanError> {
    let metrics: Vec<BoundMetric> = metric_names
        .iter()
        .enumerate()
        .map(|(index, metric_name)| metric_reference(metric_name, index))
        .collect();
    build_plan(
        &Branches::new(branches).expect("distinct homes"),
        &query(dimensions, metrics, BoundQueryParts::default()),
        &anyone(),
        model,
    )
}

/// [`derived_model`] with the metric `metric_name` changed by `change`.
pub fn changed_model(metric_name: &str, change: impl FnOnce(&mut Metric)) -> InternalModel {
    let base = derived_model();
    let mut parts = base.parts().clone();
    let metric = parts
        .metrics
        .iter_mut()
        .find(|metric| metric.name == name(metric_name))
        .expect("a metric of the model");
    change(metric);
    InternalModel::new(parts, base.hash().clone())
}
