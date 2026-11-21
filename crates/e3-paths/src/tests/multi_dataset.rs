//! Unit tests of queries that read several datasets (E3-3.1a, E3-4, QRY-4.3), named after
//! the rule they check.

use super::*;

#[test]
fn e3_4_2_a_dimension_on_another_dataset_joins_it_to_the_home_dataset() {
    let model = orders_to_customers(vec![metric("REVENUE", &["ORDERS"])]);
    let query = query(
        vec![dimension("CUSTOMERS", "COUNTRY", 0)],
        vec![metric_reference("REVENUE", 0)],
        authorized(BoundQueryParts::default()),
    );

    let branches = resolve_paths(&query, &model).expect("one path");

    let branch = only_branch(&branches);
    assert_eq!(branch.home, name("ORDERS"));
    let customers = branch
        .join_tree
        .bound_instance(&QueryElement::Dimension(0))
        .and_then(|id| branch.join_tree.instance(id))
        .expect("a join instance");
    assert_eq!(customers.step.to, name("CUSTOMERS"));
}

#[test]
fn e3_4_1_components_on_several_datasets_are_one_branch_per_home_dataset() {
    let model = model(Vec::new(), vec![metric("NET", &["RETURNS", "ORDERS"])]);
    let query = query(
        Vec::new(),
        vec![metric_reference("NET", 0)],
        BoundQueryParts::default(),
    );

    let branches = resolve_paths(&query, &model).expect("two branches");

    let found: Vec<(&str, &[ComponentKey])> = branches
        .as_slice()
        .iter()
        .map(|branch| (branch.home.as_str(), branch.components.as_slice()))
        .collect();
    assert_eq!(
        found,
        [("ORDERS", &[key(0, 1)][..]), ("RETURNS", &[key(0, 0)][..])]
    );
}

#[test]
fn e3_4_2_a_filter_on_another_dataset_joins_it_to_the_anchor() {
    let model = orders_to_customers(Vec::new());
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        Vec::new(),
        authorized(filters(vec![
            filter(&["ORDERS"], 0),
            filter(&["CUSTOMERS"], 1),
        ])),
    );

    let branches = resolve_paths(&query, &model).expect("one path");

    let branch = only_branch(&branches);
    assert_eq!(branch.home, name("ORDERS"));
    let element = QueryElement::Filter {
        filter: 1,
        dataset: name("CUSTOMERS"),
    };
    assert!(branch.join_tree.bound_instance(&element).is_some());
    assert_eq!(branch.join_tree.instances().len(), 1);
}

#[test]
fn e3_3_1_a_the_date_range_dataset_is_joined_as_a_filter_dataset() {
    let model = connected_model(
        Vec::new(),
        vec![metric("REVENUE", &["ORDERS"])],
        &[("ORDERS_CALENDAR", "ORDERS", "CALENDAR")],
    );
    let date = DateValue::new(2026, 1, 1).expect("a date");
    let parts = BoundQueryParts {
        date_range: Some(BoundDateRange {
            field: FieldPath {
                dataset: name("CALENDAR"),
                field: name("DAY"),
            },
            from: date,
            to: date,
            origin: at("/query/nonportable_extensions/0/data/date_range"),
        }),
        subgraph: subgraph(&["ORDERS_CALENDAR"]),
        ..BoundQueryParts::default()
    };
    let query = query(Vec::new(), vec![metric_reference("REVENUE", 0)], parts);

    let branches = resolve_paths(&query, &model).expect("one path");

    let tree = &only_branch(&branches).join_tree;
    let calendar = tree
        .bound_instance(&QueryElement::DateRange)
        .and_then(|id| tree.instance(id))
        .expect("a join instance");
    assert_eq!(calendar.step.relationship, name("ORDERS_CALENDAR"));
}

#[test]
fn qry_4_3_a_path_outside_the_authorized_subgraph_is_an_invariant_violation() {
    let model = model(Vec::new(), vec![metric("REVENUE", &["ORDERS"])]);
    let origin = at("/query/nonportable_extensions/0/data/paths/0");
    let path = BoundPath::new(
        name("ORDERS"),
        name("CUSTOMERS"),
        vec![name("ORDERS_TO_CUSTOMERS")],
        origin.clone(),
    )
    .expect("one relationship");
    let parts = BoundQueryParts {
        paths: vec![path],
        ..BoundQueryParts::default()
    };
    let query = query(Vec::new(), vec![metric_reference("REVENUE", 0)], parts);

    assert_eq!(
        resolve_paths(&query, &model),
        violation(InvariantViolation::UnauthorizedPathRelationship { origin })
    );
}
