//! A dataset that declares populations, for the `Population` tests (E4-3.12, POP-3).

use menura_contracts::types::common::{Grain, LogicalType};
use menura_e4_plan::{PlanError, PopulationRequest, plan_population};
use menura_expressions::typing::big_integer;
use menura_model::internal_model::{
    Dataset, InternalModel, PartialPeriod, Population, PopulationMode, RowPolicy,
};
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode, PopulationRule};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use menura_security::AdmittedSecurityContext;

use super::plan::column;
use super::{catalog, field, located, metrics, model_with, name, orders, tree};

/// The pointer of the population entry of `SUBSCRIPTIONS`.
pub const POPULATION_POINTER: &str = "/semantic_model/0/datasets/1/custom_extensions/0/data";

/// `SUBSCRIPTIONS`, which declares `populations` and has `policies`.
pub fn subscriptions(populations: Vec<Population>, policies: Vec<RowPolicy>) -> Dataset {
    let mut dataset = orders(policies);
    dataset.name = name("SUBSCRIPTIONS");
    dataset.source = "main.subscriptions".to_owned();
    dataset.fields = [
        ("CUSTOMER_ID", big_integer(), false),
        ("IS_BILLABLE", LogicalType::Boolean, true),
        ("PERIOD", LogicalType::Date, false),
        ("PLAN", LogicalType::Text, true),
        ("STORE_ID", big_integer(), true),
        ("TEAM", LogicalType::Text, true),
        ("VALID_FROM", LogicalType::Date, false),
        ("VALID_TO", LogicalType::Date, true),
    ]
    .into_iter()
    .map(|(field_name, logical_type, nullable)| field(field_name, logical_type, nullable))
    .collect();
    dataset.populations = populations;
    dataset.location = located("/semantic_model/0/datasets/1");
    dataset
}

/// The population `population_name` of `SUBSCRIPTIONS`, entity `CUSTOMER_ID` and `PLAN`.
pub fn population(
    population_name: &str,
    mode: PopulationMode,
    required_when: Option<&str>,
) -> Population {
    Population {
        name: name(population_name),
        entity: vec![name("CUSTOMER_ID"), name("PLAN")],
        mode,
        required_when: required_when.map(tree),
        location: located(POPULATION_POINTER),
    }
}

/// The `validity` mode over `VALID_FROM` and `VALID_TO`.
pub fn validity(partial_period: PartialPeriod) -> PopulationMode {
    PopulationMode::Validity {
        valid_from: name("VALID_FROM"),
        valid_to: name("VALID_TO"),
        partial_period,
    }
}

/// The monthly `periodic` mode over `PERIOD`.
pub fn periodic() -> PopulationMode {
    PopulationMode::Periodic {
        period: name("PERIOD"),
        grain: Grain::Month,
    }
}

/// The validity population `ACTIVE`, `period_end`.
pub fn active(required_when: Option<&str>) -> Population {
    population("ACTIVE", validity(PartialPeriod::PeriodEnd), required_when)
}

/// A policy of `SUBSCRIPTIONS` that needs the attribute `team`.
pub fn team_policy() -> RowPolicy {
    RowPolicy {
        name: "team_scope".to_owned(),
        condition: Some(tree("team = USER_ATTRIBUTE('team')")),
        roles: None,
        exempt_roles: None,
        location: located("/semantic_model/0/datasets/1/custom_extensions/1/data"),
    }
}

/// The model with `ORDERS` and `dataset`.
pub fn model_of(dataset: Dataset) -> InternalModel {
    model_with(vec![orders(Vec::new()), dataset], metrics())
}

/// The unit of periods: one month start per row, in a column named `PERIOD_START`.
pub fn periods() -> PlanNode {
    PlanNode::new(
        Operation::Scan {
            dataset: name("ORDERS"),
            row_identity: None,
        },
        OutputSchema::new(vec![column("PERIOD_START", LogicalType::Date, false)]),
        Origin::Model(located("/semantic_model/0/datasets/0")),
    )
}

/// The `Population` node of `population_name` for a query on `ORDERS`, without
/// `reference_time`.
pub fn plan(
    population_name: &str,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
) -> Result<PlanNode, PlanError> {
    plan_with_time(population_name, model, context, false)
}

/// The `Population` node of `population_name` for a query on `ORDERS`, with or without
/// `reference_time`.
pub fn plan_with_time(
    population_name: &str,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
    has_reference_time: bool,
) -> Result<PlanNode, PlanError> {
    plan_population(
        PopulationRequest {
            home: &name("ORDERS"),
            population: &name(population_name),
            grain: Grain::Month,
            periods: periods(),
            period: ColumnName::new("PERIOD_START"),
            has_reference_time,
        },
        context,
        model,
        &catalog(),
    )
}

/// The rule of a `Population` node.
pub fn rule(node: &PlanNode) -> &PopulationRule {
    match node.operation() {
        Operation::Population { rule, .. } => rule,
        other => panic!("not a Population node: {other:?}"),
    }
}

/// The visible rows a `Population` node reads.
pub fn visible_input(node: &PlanNode) -> &PlanNode {
    node.inputs().first().copied().expect("the visible rows")
}
