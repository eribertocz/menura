//! Matching a column list with a key: E0E-3.8, used by E0E-3.2 and E0E-3.4.

use super::super::declarations::{DatasetDeclaration, Place, WrittenName};
use super::super::validation::is_key;
use crate::names::Name;
use crate::pointer::JsonPointer;
use crate::read::Location;

fn place() -> Place {
    Place {
        pointer: JsonPointer::root(),
        location: Location { line: 1, column: 1 },
    }
}

/// A dataset whose keys are `keys`, each written as a list of names.
fn dataset(keys: &[&[&str]]) -> DatasetDeclaration<'static> {
    DatasetDeclaration {
        place: place(),
        name: WrittenName {
            text: "store_days",
            name: Name::declared("store_days"),
            place: place(),
        },
        keys: keys
            .iter()
            .map(|key| key.iter().map(|name| Name::declared(name)).collect())
            .collect(),
    }
}

fn columns(names: &[&str]) -> Vec<Name> {
    names.iter().map(|name| Name::declared(name)).collect()
}

#[test]
fn e0e_3_8_keys_match_as_sets_whatever_the_order() {
    let store_days = dataset(&[&["store_id", "date_key"]]);
    assert!(is_key(&store_days, &columns(&["store_id", "date_key"])));
    assert!(is_key(&store_days, &columns(&["date_key", "store_id"])));
}

#[test]
fn e0e_3_8_repetition_is_ignored() {
    let store_days = dataset(&[&["store_id", "date_key"]]);
    assert!(is_key(
        &store_days,
        &columns(&["date_key", "store_id", "date_key"])
    ));
    let repeated_key = dataset(&[&["store_id", "store_id"]]);
    assert!(is_key(&repeated_key, &columns(&["store_id"])));
}

#[test]
fn e0e_3_8_names_are_compared_after_normalization() {
    let store_days = dataset(&[&["store_id", "date_key"]]);
    assert!(is_key(&store_days, &columns(&["STORE_ID", "\"DATE_KEY\""])));
    assert!(!is_key(
        &store_days,
        &columns(&["store_id", "\"date_key\""])
    ));
}

#[test]
fn e0e_3_8_a_subset_superset_or_other_set_does_not_match() {
    let store_days = dataset(&[&["store_id", "date_key"], &["opening_hours"]]);
    assert!(!is_key(&store_days, &columns(&["store_id"])));
    assert!(!is_key(
        &store_days,
        &columns(&["store_id", "date_key", "opening_hours"])
    ));
    assert!(is_key(&store_days, &columns(&["opening_hours"])));
    assert!(!is_key(&dataset(&[]), &columns(&["store_id"])));
}
