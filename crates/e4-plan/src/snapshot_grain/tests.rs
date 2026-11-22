//! Unit tests of the grain check of a semi-additive `over` (ADD-4.9, E4-7.8).

use menura_contracts::types::common::{Grain, LogicalType, TimestampPrecision};

use menura_model::internal_model::{FieldPath, NormalizedName};
use menura_representations::bound_query::BoundDimensionKind;

use super::{OverGrain, first_finer};

fn timestamp() -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: false,
    }
}

#[test]
fn add_4_9_a_grain_finer_than_the_snapshot_grain_splits_it() {
    let week = OverGrain::of(Some(Grain::Week), &LogicalType::Date);

    assert_eq!(week, Some(OverGrain::Grain(Grain::Week)));
    assert!(week.is_some_and(|grain| grain.is_finer_than(Grain::Month)));
}

#[test]
fn add_4_9_the_snapshot_grain_or_a_coarser_one_does_not_split_it() {
    for grain in [Grain::Month, Grain::Quarter, Grain::Year] {
        assert!(!OverGrain::Grain(grain).is_finer_than(Grain::Month));
    }
}

#[test]
fn e4_7_8_a_plain_date_has_grain_day() {
    let plain = OverGrain::of(None, &LogicalType::Date);

    assert_eq!(plain, Some(OverGrain::Grain(Grain::Day)));
    assert!(plain.is_some_and(|grain| !grain.is_finer_than(Grain::Day)));
    assert!(plain.is_some_and(|grain| grain.is_finer_than(Grain::Month)));
}

#[test]
fn e4_7_8_a_plain_timestamp_has_its_source_precision() {
    let plain = OverGrain::of(None, &timestamp());

    assert_eq!(plain, Some(OverGrain::SourcePrecision));
    assert!(plain.is_some_and(|grain| grain.is_finer_than(Grain::Day)));
}

#[test]
fn e4_7_8_a_grained_timestamp_has_its_grain() {
    let monthly = OverGrain::of(Some(Grain::Month), &timestamp());

    assert_eq!(monthly, Some(OverGrain::Grain(Grain::Month)));
}

#[test]
fn e4_7_8_a_field_that_is_not_time_has_no_grain() {
    assert_eq!(OverGrain::of(None, &LogicalType::Text), None);
}

fn path(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: NormalizedName::new(dataset),
        field: NormalizedName::new(field),
    }
}

fn dimension(field: FieldPath, time_grain: Option<Grain>) -> BoundDimensionKind {
    BoundDimensionKind::Field { field, time_grain }
}

#[test]
fn e4_03_the_first_dimension_of_over_finer_than_the_snapshot_grain_is_reported() {
    let over = path("DEBT", "PERIOD");
    let dimensions = [
        dimension(path("DEBT", "CATEGORY"), None),
        dimension(over.clone(), Some(Grain::Year)),
        dimension(over.clone(), Some(Grain::Week)),
        dimension(over.clone(), None),
    ];

    let found = first_finer(dimensions.iter(), &over, &LogicalType::Date, Grain::Month);

    assert_eq!(found, Some(2));
}

#[test]
fn add_4_10_a_query_without_a_dimension_of_over_is_never_split() {
    let over = path("DEBT", "PERIOD");
    let dimensions = [dimension(path("DEBT", "OPENED_ON"), Some(Grain::Day))];

    let found = first_finer(dimensions.iter(), &over, &LogicalType::Date, Grain::Month);

    assert_eq!(found, None);
}
