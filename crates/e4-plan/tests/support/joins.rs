//! A model with relationships, and branches built by hand, for the join tests (E4-4.0c).

use menura_contracts::types::common::{Grain, LogicalType};
use menura_expressions::typing::{big_integer, decimal};
use menura_model::internal_model::{
    Dataset, Descriptions, FieldPath, InternalModel, Relationship, RowPolicy,
};
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundFilter};
use menura_representations::names::OutputName;
use menura_representations::paths::{
    Branch, BranchKind, Branches, ComponentKey, ElementPath, JoinStep, JoinTree, QueryElement,
};

use super::{at, field, located, metrics, model_with, name, orders, tree};

/// The fields of a dataset: name, logical type, nullable.
type Fields = Vec<(&'static str, LogicalType, bool)>;

fn dataset(dataset_name: &str, index: usize, fields: Fields) -> Dataset {
    let mut dataset = orders(Vec::new());
    dataset.name = name(dataset_name);
    dataset.source = format!("main.{}", dataset_name.to_lowercase());
    dataset.fields = fields
        .into_iter()
        .map(|(field_name, logical_type, nullable)| field(field_name, logical_type, nullable))
        .collect();
    dataset.location = located(&format!("/semantic_model/0/datasets/{index}"));
    dataset
}

/// `ORDERS` with a customer and two dates.
pub fn sales_orders() -> Dataset {
    dataset(
        "ORDERS",
        0,
        vec![
            ("AMOUNT", decimal(12, 2), true),
            ("CUSTOMER_ID", big_integer(), false),
            ("ORDER_DATE", LogicalType::Date, false),
            ("SHIP_DATE", LogicalType::Date, true),
            ("STATUS", LogicalType::Text, false),
        ],
    )
}

/// `CUSTOMERS`, with `policies`.
pub fn customers(policies: Vec<RowPolicy>) -> Dataset {
    let mut customers = dataset(
        "CUSTOMERS",
        1,
        vec![
            ("CUSTOMER_ID", big_integer(), false),
            ("REGION_ID", big_integer(), true),
            ("SEGMENT", LogicalType::Text, false),
        ],
    );
    customers.row_policies = policies;
    customers
}

fn regions() -> Dataset {
    dataset(
        "REGIONS",
        2,
        vec![
            ("NAME", LogicalType::Text, false),
            ("REGION_ID", big_integer(), false),
        ],
    )
}

fn calendar() -> Dataset {
    dataset(
        "CALENDAR",
        3,
        vec![
            ("DAY", LogicalType::Date, false),
            ("MONTH_NAME", LogicalType::Text, false),
        ],
    )
}

/// The location of the relationship at `index`.
pub fn relationship_pointer(index: usize) -> String {
    format!("/semantic_model/0/relationships/{index}")
}

fn relationship(
    index: usize,
    relationship_name: &str,
    (from, from_column): (&str, &str),
    (to, to_column): (&str, &str),
) -> Relationship {
    Relationship {
        name: name(relationship_name),
        from: name(from),
        to: name(to),
        from_columns: vec![name(from_column)],
        to_columns: vec![name(to_column)],
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: located(&relationship_pointer(index)),
    }
}

/// The model of `ORDERS`, `CUSTOMERS` (with `customer_policies`), `REGIONS` and `CALENDAR`,
/// with the relationships `ORDERS_TO_CUSTOMERS`, `CUSTOMERS_TO_REGIONS`, `ORDERED_ON` and
/// `SHIPPED_ON`, and the metrics of [`metrics`].
pub fn sales_model_with(home: Dataset, customer_policies: Vec<RowPolicy>) -> InternalModel {
    let base = model_with(
        vec![calendar(), customers(customer_policies), home, regions()],
        metrics(),
    );
    let mut parts = base.parts().clone();
    parts.relationships = vec![
        relationship(
            1,
            "CUSTOMERS_TO_REGIONS",
            ("CUSTOMERS", "REGION_ID"),
            ("REGIONS", "REGION_ID"),
        ),
        relationship(
            2,
            "ORDERED_ON",
            ("ORDERS", "ORDER_DATE"),
            ("CALENDAR", "DAY"),
        ),
        relationship(
            0,
            "ORDERS_TO_CUSTOMERS",
            ("ORDERS", "CUSTOMER_ID"),
            ("CUSTOMERS", "CUSTOMER_ID"),
        ),
        relationship(
            3,
            "SHIPPED_ON",
            ("ORDERS", "SHIP_DATE"),
            ("CALENDAR", "DAY"),
        ),
    ];
    InternalModel::new(parts, base.hash().clone())
}

/// [`sales_model_with`] [`sales_orders`] and no customer policy.
pub fn sales_model() -> InternalModel {
    sales_model_with(sales_orders(), Vec::new())
}

/// The step across `relationship` from `from` to `to`.
pub fn step(relationship: &str, from: &str, to: &str) -> JoinStep {
    JoinStep {
        relationship: name(relationship),
        from: name(from),
        to: name(to),
    }
}

/// `ORDERS` to `CUSTOMERS`.
pub fn to_customers() -> JoinStep {
    step("ORDERS_TO_CUSTOMERS", "ORDERS", "CUSTOMERS")
}

/// `CUSTOMERS` to `REGIONS`.
pub fn to_regions() -> JoinStep {
    step("CUSTOMERS_TO_REGIONS", "CUSTOMERS", "REGIONS")
}

/// `ORDERS` to `CALENDAR` by order date.
pub fn ordered_on() -> JoinStep {
    step("ORDERED_ON", "ORDERS", "CALENDAR")
}

/// `ORDERS` to `CALENDAR` by ship date.
pub fn shipped_on() -> JoinStep {
    step("SHIPPED_ON", "ORDERS", "CALENDAR")
}

/// The path of `element` through `steps`.
pub fn path(element: QueryElement, steps: Vec<JoinStep>) -> ElementPath {
    ElementPath { element, steps }
}

/// The single branch on `ORDERS` with `components` components of the first metric (none for
/// a dimension-only query) and the join tree of `paths`.
pub fn joined_branches(components: usize, paths: Vec<ElementPath>) -> Branches {
    let join_tree = JoinTree::from_paths(paths);
    if components == 0 {
        return Branches::dimension_only(name("ORDERS"), join_tree);
    }
    Branches::new(vec![Branch {
        home: name("ORDERS"),
        kind: BranchKind::Plain,
        components: (0..components)
            .map(|component| ComponentKey {
                metric: 0,
                component,
            })
            .collect(),
        join_tree,
    }])
    .expect("one branch")
}

/// The dimension `<dataset>.<field>` at `/query/dimensions/<index>`, with an optional grain.
pub fn joined_dimension(
    dataset: &str,
    field: &str,
    grain: Option<Grain>,
    index: usize,
) -> BoundDimension {
    BoundDimension {
        kind: BoundDimensionKind::Field {
            field: FieldPath {
                dataset: name(dataset),
                field: name(field),
            },
            time_grain: grain,
            time_grain_origin: None,
        },
        output_name: OutputName::new(&format!("{}_{index}", field.to_lowercase())),
        origin: at(&format!("/query/dimensions/{index}")),
    }
}

/// The filter `text` reading `datasets`, at `/query/filters/<index>`.
pub fn joined_filter(text: &str, datasets: &[&str], index: usize) -> BoundFilter {
    BoundFilter {
        expression: tree(text),
        datasets: datasets.iter().map(|dataset| name(dataset)).collect(),
        origin: at(&format!("/query/filters/{index}")),
    }
}
