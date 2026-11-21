//! A relationship key that names a source column rather than a field (E0D-3.1): the `Join`
//! reads that column through the `Scan` of the dataset it belongs to (E4-3.2, E4-4.0c,
//! E4-4.0g, issue #339).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::catalog::{CatalogSchemaDocument, Column, RelationSchema};
use menura_contracts::types::common::LogicalType;
use menura_expressions::typing::big_integer;
use menura_model::internal_model::{Dataset, InternalModel};
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::PlanNode;
use menura_representations::paths::QueryElement;
use support::join_plan::{aggregate_keys, condition, join_parts, scan_dataset};
use support::joins::{joined_branches, joined_dimension, path, sales_model, to_customers};
use support::plan::{column, column_names, column_reference, kinds, node};
use support::{anyone, catalog, metric_reference, query};

/// The catalog of [`catalog`], with the relation `main.customers` and its column
/// `customer_id`, and `customer_id` added to `main.orders`.
fn catalog_with_keys() -> CatalogSchemaDocument {
    let key = || Column {
        name: "customer_id".to_owned(),
        logical_type: big_integer(),
        physical_type: "unused".to_owned(),
        nullable: false,
    };
    let mut catalog = catalog();
    for relation in &mut catalog.relations {
        relation.columns.push(key());
    }
    catalog.relations.push(RelationSchema {
        source: "main.customers".to_owned(),
        columns: vec![key()],
    });
    catalog
}

/// [`sales_model`] without the field `CUSTOMER_ID` in the datasets `without`, so that the key
/// of `ORDERS_TO_CUSTOMERS` resolves there to the source column (E0D-3.1).
fn model_without_key_field(without: &[&str]) -> InternalModel {
    let base = sales_model();
    let mut parts = base.parts().clone();
    for dataset in &mut parts.datasets {
        if without.contains(&dataset.name.as_str()) {
            drop_key_field(dataset);
        }
    }
    InternalModel::new(parts, base.hash().clone())
}

fn drop_key_field(dataset: &mut Dataset) {
    dataset
        .fields
        .retain(|field| field.name.as_str() != "CUSTOMER_ID");
}

/// The plan of `CUSTOMERS.SEGMENT` and `TOTAL_AMOUNT` through `ORDERS_TO_CUSTOMERS`, over
/// [`model_without_key_field`] `without`.
fn segment_plan_without(without: &[&str]) -> PlanNode {
    let branches = joined_branches(
        1,
        vec![path(QueryElement::Dimension(0), vec![to_customers()])],
    );
    let bound = query(
        vec![joined_dimension("CUSTOMERS", "SEGMENT", None, 0)],
        vec![metric_reference("TOTAL_AMOUNT", 0)],
        BoundQueryParts::default(),
    );
    menura_e4_plan::build_plan(
        &branches,
        &bound,
        &anyone(),
        &model_without_key_field(without),
        &catalog_with_keys(),
    )
    .expect("a plan")
}

#[test]
fn e0d_3_1_a_join_reads_a_to_column_that_is_a_source_column_through_its_instance() {
    let plan = segment_plan_without(&["CUSTOMERS"]);

    assert_eq!(
        kinds(&plan),
        ["Project", "Aggregate", "Deduplicate", "Join", "Scan"]
    );
    let (home, joined, on, prefix, _) = join_parts(node(&plan, "Join"));
    assert_eq!(prefix, "ORDERS_TO_CUSTOMERS.");
    assert_eq!(scan_dataset(home), "ORDERS");
    assert_eq!(scan_dataset(joined), "CUSTOMERS");
    assert_eq!(
        on,
        [condition(
            "CUSTOMER_ID",
            "ORDERS_TO_CUSTOMERS.menura.source.CUSTOMER_ID"
        )]
    );
    assert_eq!(
        joined.schema().columns(),
        [
            column("ORDERS_TO_CUSTOMERS.SEGMENT", LogicalType::Text, false),
            column(
                "ORDERS_TO_CUSTOMERS.menura.source.CUSTOMER_ID",
                big_integer(),
                false
            ),
        ]
    );
    assert_eq!(
        aggregate_keys(&plan),
        [column_reference("ORDERS_TO_CUSTOMERS.SEGMENT")]
    );
}

#[test]
fn e0d_3_1_a_join_reads_a_from_column_that_is_a_source_column_of_the_home_dataset() {
    let plan = segment_plan_without(&["ORDERS"]);

    let (home, _, on, ..) = join_parts(node(&plan, "Join"));
    assert_eq!(
        on,
        [condition(
            "menura.source.CUSTOMER_ID",
            "ORDERS_TO_CUSTOMERS.CUSTOMER_ID"
        )]
    );
    assert!(
        column_names(home).contains(&"menura.source.CUSTOMER_ID"),
        "the home Scan reads the source column"
    );
}

#[test]
fn e0d_3_1_e4_4_0c_both_key_columns_can_be_source_columns() {
    let plan = segment_plan_without(&["ORDERS", "CUSTOMERS"]);

    let (.., on, _, _) = join_parts(node(&plan, "Join"));
    assert_eq!(
        on,
        [condition(
            "menura.source.CUSTOMER_ID",
            "ORDERS_TO_CUSTOMERS.menura.source.CUSTOMER_ID"
        )]
    );
}
