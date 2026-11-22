//! Unit tests of the parts of a `Snapshot` rule (ADD-3.2, ADD-4.7, ADD-4.10).

use menura_model::internal_model::{
    FieldPath, NormalizedName, NullHandling, SemiAdditive, SnapshotAggregation,
};

use super::{entity_column, snapshot_fields};

fn path(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: NormalizedName::new(dataset),
        field: NormalizedName::new(field),
    }
}

fn declaration() -> SemiAdditive {
    SemiAdditive {
        over: path("STOCK", "WAREHOUSE"),
        entity: vec![path("STOCK", "PRODUCT"), path("STOCK", "LOT")],
        aggregation: SnapshotAggregation::Maximum,
        nulls: NullHandling::Keep,
        snapshot_grain: None,
        population: None,
        missing: None,
        unexpected: None,
    }
}

#[test]
fn e4_3_13_entity_columns_are_named_by_position() {
    assert_eq!(entity_column(0).as_str(), "entity_1");
    assert_eq!(entity_column(1).as_str(), "entity_2");
}

#[test]
fn e4_7_1_the_snapshot_reads_over_then_the_entity_fields() {
    let fields: Vec<String> = snapshot_fields(&declaration())
        .iter()
        .map(|field| field.as_str().to_owned())
        .collect();

    assert_eq!(fields, ["WAREHOUSE", "PRODUCT", "LOT"]);
}
