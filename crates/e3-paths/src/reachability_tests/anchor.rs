//! Unit tests of the anchor of a dimension-only query (E3-5), named after the rule they
//! check.

use super::*;

#[test]
fn e3_5_1_a_dimension_dataset_that_reaches_the_others_is_an_anchor() {
    let query = authorized_query(
        vec![
            dimension("CUSTOMERS", "NAME", 0),
            dimension("ORDERS", "STATUS", 1),
        ],
        Vec::new(),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    let branches = resolve_paths(&query, &fixture()).expect("one anchor");

    let homes: Vec<&str> = branches
        .as_slice()
        .iter()
        .map(|branch| branch.home.as_str())
        .collect();
    assert_eq!(homes, ["ORDERS"]);
}

#[test]
fn e3_5_1_without_an_anchor_is_e_no_anchor_at_the_dimensions() {
    let query = authorized_query(
        vec![
            dimension("CUSTOMERS", "NAME", 0),
            dimension("WAREHOUSES", "CITY", 1),
        ],
        Vec::new(),
        vec![filter(&["RETURNS"], 0)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::NoAnchor, "/query/dimensions"))
    );
}

#[test]
fn e3_5_1_more_than_one_anchor_is_e_ambiguous_path_at_the_dimensions() {
    let query = authorized_query(
        vec![
            dimension("ACCOUNTS", "PLAN", 0),
            dimension("PROFILES", "SEGMENT", 1),
        ],
        Vec::new(),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::AmbiguousPath, "/query/dimensions"))
    );
}

#[test]
fn e3_5_2_a_filter_dataset_the_anchor_cannot_reach_is_located_at_its_filter() {
    let query = authorized_query(
        vec![dimension("ORDERS", "STATUS", 0)],
        Vec::new(),
        vec![filter(&["RETURNS"], 0)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(
            PathCode::UnsupportedFilterPath,
            "/query/filters/0"
        ))
    );
}

#[test]
fn e3_5_2_a_date_range_the_anchor_cannot_reach_is_located_at_its_entry() {
    let query = authorized_query(
        vec![dimension("ORDERS", "STATUS", 0)],
        Vec::new(),
        Vec::new(),
        Some(date_range_on("RETURNS")),
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::UnsupportedFilterPath, DATE_RANGE))
    );
}

#[test]
fn e3_5_2_with_several_anchors_the_filter_check_comes_before_the_ambiguity() {
    let query = authorized_query(
        vec![
            dimension("ACCOUNTS", "PLAN", 0),
            dimension("PROFILES", "SEGMENT", 1),
        ],
        Vec::new(),
        vec![filter(&["WAREHOUSES"], 0)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(
            PathCode::UnsupportedFilterPath,
            "/query/filters/0"
        ))
    );
}
