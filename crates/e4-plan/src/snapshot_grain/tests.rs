//! Unit tests of the grain check of a semi-additive `over` (ADD-4.9, E4-7.8).

use menura_contracts::types::common::{Grain, LogicalType, TimestampPrecision};

use menura_model::expression::{DateUnit, Expression, Identifier, Reference};
use menura_model::internal_model::{FieldPath, NormalizedName};
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind};
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;

use super::{OverGrain, SnapshotAxis, location, splits};

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

    assert_eq!(plain, Some(OverGrain::FinerThanDay));
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

fn at(pointer: &str) -> QueryLocation {
    QueryLocation::new(pointer, None)
}

fn dimension(kind: BoundDimensionKind) -> BoundDimension {
    BoundDimension {
        kind,
        output_name: OutputName::new("period"),
        origin: at("/query/dimensions/0"),
    }
}

fn field(field: FieldPath, time_grain: Option<Grain>, entry: Option<&str>) -> BoundDimensionKind {
    BoundDimensionKind::Field {
        field,
        time_grain,
        time_grain_origin: entry.map(at),
    }
}

fn sql(dataset: &str, expression: Expression) -> BoundDimensionKind {
    BoundDimensionKind::Expression {
        dataset: NormalizedName::new(dataset),
        expression,
    }
}

fn period_reference() -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::regular("period").expect("a regular identifier"),
    })
}

fn truncated(unit: DateUnit) -> Expression {
    Expression::DateTrunc {
        unit,
        operand: Box::new(period_reference()),
    }
}

#[test]
fn e4_03_only_a_dimension_of_over_finer_than_the_snapshot_grain_splits_it() {
    let over = path("DEBT", "PERIOD");
    let axis = SnapshotAxis {
        over: &over,
        over_type: &LogicalType::Date,
        snapshot_grain: Grain::Month,
    };

    assert!(splits(&field(over.clone(), Some(Grain::Week), None), &axis));
    assert!(splits(&field(over.clone(), None, None), &axis));
    assert!(!splits(
        &field(over.clone(), Some(Grain::Year), None),
        &axis
    ));
    assert!(!splits(
        &field(path("DEBT", "OPENED_ON"), Some(Grain::Day), None),
        &axis
    ));
}

#[test]
fn e4_7_8_date_trunc_of_over_in_the_sql_form_counts_as_its_grain() {
    let over = path("DEBT", "PERIOD");
    let axis = SnapshotAxis {
        over: &over,
        over_type: &LogicalType::Date,
        snapshot_grain: Grain::Month,
    };

    assert!(splits(&sql("DEBT", truncated(DateUnit::Day)), &axis));
    assert!(splits(&sql("DEBT", period_reference()), &axis));
    assert!(!splits(&sql("DEBT", truncated(DateUnit::Quarter)), &axis));
    assert!(!splits(&sql("CUSTOMERS", truncated(DateUnit::Day)), &axis));
}

#[test]
fn e4_03_is_located_at_the_time_grains_entry_else_at_the_dimension() {
    let entry = "/query/nonportable_extensions/0/data/time_grains/0";
    let grained = dimension(field(
        path("DEBT", "PERIOD"),
        Some(Grain::Week),
        Some(entry),
    ));
    let plain = dimension(field(path("DEBT", "PERIOD"), None, None));
    let select_item = BoundDimension {
        origin: QueryLocation::new("/sql", None),
        ..dimension(sql("DEBT", truncated(DateUnit::Day)))
    };

    assert_eq!(location(&grained).pointer(), entry);
    assert_eq!(location(&plain).pointer(), "/query/dimensions/0");
    assert_eq!(location(&select_item).pointer(), "/sql");
}

#[test]
fn add_4_10_a_query_without_a_dimension_of_over_is_never_split() {
    let over = path("DEBT", "PERIOD");
    let axis = SnapshotAxis {
        over: &over,
        over_type: &LogicalType::Date,
        snapshot_grain: Grain::Month,
    };

    assert!(!splits(
        &field(path("DEBT", "OPENED_ON"), None, None),
        &axis
    ));
}
