//! Models with relationships and row policies on other datasets than the home dataset, for
//! the propagation tests (E4-6.5 to E4-6.10).

use menura_contracts::types::common::LogicalType;
use menura_expressions::typing::{big_integer, decimal};
use menura_model::expression::Expression;
use menura_model::internal_model::{
    AccessMode, Dataset, DatasetAccess, Descriptions, GraphEdge, InternalModel, Relationship,
    RoleName, RowPolicy,
};
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::paths::{Branch, Branches, ComponentKey, JoinStep, JoinTree};

use super::plan::{chain, column_names, kind};
use super::{field, located, metrics, model_with, name, orders};

/// One relationship of a test model: name, `from` dataset and column, `to` dataset and
/// column.
pub type Link = (
    &'static str,
    (&'static str, &'static str),
    (&'static str, &'static str),
);

fn dataset(dataset_name: &str, index: usize, fields: &[(&str, LogicalType, bool)]) -> Dataset {
    let mut dataset = orders(Vec::new());
    dataset.name = name(dataset_name);
    dataset.source = format!("main.{}", dataset_name.to_lowercase());
    dataset.fields = fields
        .iter()
        .map(|(field_name, logical_type, nullable)| {
            field(field_name, logical_type.clone(), *nullable)
        })
        .collect();
    dataset.location = located(&format!("/semantic_model/0/datasets/{index}"));
    dataset
}

/// `ORDERS`, the home dataset: an amount and three foreign keys, all nullable.
pub fn home_orders() -> Dataset {
    dataset(
        "ORDERS",
        0,
        &[
            ("AMOUNT", decimal(12, 2), true),
            ("BILLED_TO", big_integer(), true),
            ("CUSTOMER_ID", big_integer(), true),
            ("REGION", LogicalType::Text, true),
            ("SHIPPED_TO", big_integer(), true),
            ("WAREHOUSE_ID", big_integer(), true),
        ],
    )
}

/// `CUSTOMERS`: a store and an owner key.
pub fn customers() -> Dataset {
    dataset(
        "CUSTOMERS",
        1,
        &[
            ("CUSTOMER_ID", big_integer(), false),
            ("REGION", LogicalType::Text, true),
            ("STORE_ID", big_integer(), true),
            ("TEAM", LogicalType::Text, true),
        ],
    )
}

/// `STORES`: a region and an owner, who is a customer.
pub fn stores() -> Dataset {
    dataset(
        "STORES",
        2,
        &[
            ("OWNER_ID", big_integer(), true),
            ("REGION", LogicalType::Text, true),
            ("STORE_ID", big_integer(), false),
        ],
    )
}

/// `WAREHOUSES`: a store.
pub fn warehouses() -> Dataset {
    dataset(
        "WAREHOUSES",
        3,
        &[
            ("STORE_ID", big_integer(), true),
            ("WAREHOUSE_ID", big_integer(), false),
        ],
    )
}

/// `dataset` with `policies`.
pub fn with_policies(mut dataset: Dataset, policies: Vec<RowPolicy>) -> Dataset {
    dataset.row_policies = policies;
    dataset
}

/// `dataset` readable only by `roles`.
pub fn readable_by(mut dataset: Dataset, roles: &[&str]) -> Dataset {
    dataset.access = Some(DatasetAccess {
        mode: AccessMode::Roles(roles.iter().map(|role| RoleName::new(role)).collect()),
        fields: Vec::new(),
    });
    dataset
}

/// `ORDERS` to `CUSTOMERS`.
pub const ORDERS_TO_CUSTOMERS: Link = (
    "ORDERS_TO_CUSTOMERS",
    ("ORDERS", "CUSTOMER_ID"),
    ("CUSTOMERS", "CUSTOMER_ID"),
);

/// `CUSTOMERS` to `STORES`.
pub const CUSTOMERS_TO_STORES: Link = (
    "CUSTOMERS_TO_STORES",
    ("CUSTOMERS", "STORE_ID"),
    ("STORES", "STORE_ID"),
);

/// `STORES` back to `CUSTOMERS`, by owner: closes a cycle with [`CUSTOMERS_TO_STORES`].
pub const STORE_OWNER: Link = (
    "STORE_OWNER",
    ("STORES", "OWNER_ID"),
    ("CUSTOMERS", "CUSTOMER_ID"),
);

/// `ORDERS` to `CUSTOMERS`, by the billed customer.
pub const BILLED_TO: Link = (
    "BILLED_TO",
    ("ORDERS", "BILLED_TO"),
    ("CUSTOMERS", "CUSTOMER_ID"),
);

/// `ORDERS` to `CUSTOMERS`, by the shipped-to customer.
pub const SHIPPED_TO: Link = (
    "SHIPPED_TO",
    ("ORDERS", "SHIPPED_TO"),
    ("CUSTOMERS", "CUSTOMER_ID"),
);

/// `ORDERS` to `WAREHOUSES`.
pub const ORDERS_TO_WAREHOUSES: Link = (
    "ORDERS_TO_WAREHOUSES",
    ("ORDERS", "WAREHOUSE_ID"),
    ("WAREHOUSES", "WAREHOUSE_ID"),
);

/// `WAREHOUSES` to `STORES`: with [`ORDERS_TO_CUSTOMERS`] and [`CUSTOMERS_TO_STORES`], a
/// diamond from `ORDERS` to `STORES`.
pub const WAREHOUSES_TO_STORES: Link = (
    "WAREHOUSES_TO_STORES",
    ("WAREHOUSES", "STORE_ID"),
    ("STORES", "STORE_ID"),
);

/// A model with `datasets`, the relationships `links` (all usable, so all in the graph) and
/// the metrics of [`metrics`].
pub fn linked_model(datasets: Vec<Dataset>, links: &[Link]) -> InternalModel {
    let base = model_with(datasets, metrics());
    let mut parts = base.parts().clone();
    let mut sorted: Vec<&Link> = links.iter().collect();
    sorted.sort_by_key(|link| link.0);
    parts.relationships = sorted
        .iter()
        .enumerate()
        .map(
            |(index, (relationship, (from, from_column), (to, to_column)))| Relationship {
                name: name(relationship),
                from: name(from),
                to: name(to),
                from_columns: vec![name(from_column)],
                to_columns: vec![name(to_column)],
                descriptions: Descriptions::default(),
                query_errors: Vec::new(),
                location: located(&format!("/semantic_model/0/relationships/{index}")),
            },
        )
        .collect();
    parts.graph.edges = sorted
        .iter()
        .map(|(relationship, (from, _), (to, _))| GraphEdge {
            relationship: name(relationship),
            from: name(from),
            to: name(to),
            one_to_one: false,
        })
        .collect();
    InternalModel::new(parts, base.hash().clone())
}

/// The step across `link`.
pub fn across(link: Link) -> JoinStep {
    let (relationship, (from, _), (to, _)) = link;
    JoinStep {
        relationship: name(relationship),
        from: name(from),
        to: name(to),
    }
}

/// The single branch on `ORDERS` with one component of the first metric and no join.
pub fn home_branch() -> Branches {
    Branches::new(vec![Branch {
        home: name("ORDERS"),
        components: vec![ComponentKey {
            metric: 0,
            component: 0,
        }],
        join_tree: JoinTree::default(),
    }])
    .expect("one branch")
}

/// A one-line outline of the visible rows under `node`: each `Scan` with its dataset and
/// columns, each `Filter`, and each `SemiJoin` with its relationship, keys, `related_all`,
/// `related_visible` and input.
pub fn outline(node: &PlanNode) -> String {
    match node.operation() {
        Operation::Scan { dataset, .. } => format!(
            "Scan {} [{}]",
            dataset.as_str(),
            column_names(node).join(", ")
        ),
        Operation::Filter { input, .. } => format!("Filter({})", outline(input)),
        Operation::SemiJoin {
            input,
            relationship,
            on,
            related_all,
            related_visible,
        } => {
            let keys: Vec<String> = on
                .iter()
                .map(|condition| {
                    format!("{}={}", condition.left.as_str(), condition.right.as_str())
                })
                .collect();
            format!(
                "SemiJoin {} on {} all({}) visible({}) over({})",
                relationship.as_str(),
                keys.join(", "),
                outline(related_all),
                outline(related_visible),
                outline(input)
            )
        }
        _ => format!("{}({})", kind(node), outline_first_input(node)),
    }
}

fn outline_first_input(node: &PlanNode) -> String {
    node.inputs()
        .first()
        .map_or_else(String::new, |input| outline(input))
}

/// The predicates of every `Filter` under `node`, inputs before `related_all` before
/// `related_visible`, depth first.
pub fn filter_predicates(node: &PlanNode) -> Vec<Expression> {
    let mut found = Vec::new();
    if let Operation::Filter { predicate, .. } = node.operation() {
        found.push(predicate.clone());
    }
    for input in node.inputs() {
        found.extend(filter_predicates(input));
    }
    found
}

/// The relationships of the `SemiJoin` nodes over the `Scan` of `plan`, from the `Scan` up.
pub fn semi_join_relationships(plan: &PlanNode) -> Vec<String> {
    let mut found: Vec<String> = chain(plan)
        .into_iter()
        .filter_map(|node| match node.operation() {
            Operation::SemiJoin { relationship, .. } => Some(relationship.as_str().to_owned()),
            _ => None,
        })
        .collect();
    found.reverse();
    found
}
