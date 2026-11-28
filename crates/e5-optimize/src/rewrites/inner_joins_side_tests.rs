//! Condition (2) of E5-3.3a with the `from_time` of a temporal relationship (VAL-2.15).

use std::collections::BTreeSet;

use menura_representations::logical_plan::JoinCondition;
use menura_representations::names::ColumnName;

use super::{Side, from_side_reads_every_row};

/// The key pair on `CUSTOMER_ID`.
fn keys() -> Vec<JoinCondition> {
    vec![JoinCondition {
        left: ColumnName::new("CUSTOMER_ID"),
        right: ColumnName::new("SALE_CUSTOMER.CUSTOMER_ID"),
    }]
}

/// The home side, with `CUSTOMER_ID` and `SOLD_ON`, then a side joined by a `left` join with
/// `STORE.SOLD_ON`.
fn sides() -> Vec<Side<'static>> {
    vec![
        Side {
            columns: BTreeSet::from(["CUSTOMER_ID", "SOLD_ON"]),
            reads_every_row: true,
        },
        Side {
            columns: BTreeSet::from(["STORE.SOLD_ON"]),
            reads_every_row: false,
        },
    ]
}

#[test]
fn val_2_15_a_from_time_on_the_side_of_the_keys_allows_inner() {
    let time = ColumnName::new("SOLD_ON");
    assert!(from_side_reads_every_row(&keys(), Some(&time), &sides()));
}

#[test]
fn val_2_15_a_from_time_on_another_side_keeps_the_join_left() {
    let time = ColumnName::new("STORE.SOLD_ON");
    assert!(!from_side_reads_every_row(&keys(), Some(&time), &sides()));
}

#[test]
fn val_2_15_a_from_time_on_no_side_keeps_the_join_left() {
    let time = ColumnName::new("UNKNOWN");
    assert!(!from_side_reads_every_row(&keys(), Some(&time), &sides()));
}
