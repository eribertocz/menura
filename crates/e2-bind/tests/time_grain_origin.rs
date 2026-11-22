//! The bound query keeps where each `time_grains` entry is written, so that E4 can locate
//! `E4-03` at it (E4-7.8).

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_representations::bound_query::BoundDimensionKind;
use menura_representations::query_tree::QueryTreeParts;

use support::bind_query;
use support::model::model;
use support::query::{dimension, time_grain, tree};

const GRAIN: &str = "/query/nonportable_extensions/0/data/time_grains/0";

fn grain_origin(kind: &BoundDimensionKind) -> Option<&str> {
    match kind {
        BoundDimensionKind::Field {
            time_grain_origin, ..
        } => time_grain_origin
            .as_ref()
            .map(menura_representations::origin::QueryLocation::pointer),
        BoundDimensionKind::Expression { .. } => None,
    }
}

#[test]
fn e4_7_8_a_grained_dimension_keeps_the_location_of_its_time_grains_entry() {
    let query = tree(
        vec![
            dimension(0, "orders.ordered_at"),
            dimension(1, "orders.region"),
        ],
        vec![],
        QueryTreeParts {
            time_grains: vec![time_grain(GRAIN, "orders.ordered_at", Grain::Month)],
            ..QueryTreeParts::default()
        },
    );

    let bound = bind_query(&query, &model(), &["analyst"]).expect("a bound query");

    let origins: Vec<Option<&str>> = bound
        .columns()
        .dimensions()
        .map(|dimension| grain_origin(&dimension.kind))
        .collect();
    assert_eq!(origins, [Some(GRAIN), None]);
}
