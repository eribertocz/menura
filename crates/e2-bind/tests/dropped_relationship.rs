//! E2-3.5 for a relationship E0 drops (`E0E-01`, `E0E-02`, reported as `W0E-04`): it is not in
//! the model, so a `paths` entry that names it is `E_NAME_NOT_FOUND` at that entry (E0-4 table).

#[cfg(test)]
mod support;

use menura_e2_bind::BindCode;
use menura_model::internal_model::{DiagnosticCode, ModelParts};
use menura_representations::query_tree::QueryTreeParts;

use support::bind_error;
use support::model::{model_with, name};
use support::query::{dimension, metric, path, tree};

const ANALYST: &[&str] = &["analyst"];

const ENTRY: &str = "/query/nonportable_extensions/0/data/paths/0";

/// Marks the relationship `relationship` of `parts` as E0 records one it drops.
fn drop_relationship(parts: &mut ModelParts, relationship: &str) {
    for found in &mut parts.relationships {
        if found.name == name(relationship) {
            found.query_errors = vec![DiagnosticCode::new("E_NAME_NOT_FOUND")];
        }
    }
}

#[test]
fn e0e_01_a_path_naming_a_dropped_relationship_is_name_not_found_at_the_entry() {
    let model = model_with(|parts| drop_relationship(parts, "ORDERS_TO_CUSTOMERS"));
    let parts = QueryTreeParts {
        paths: vec![path(ENTRY, "orders", "customers", &["orders_to_customers"])],
        ..QueryTreeParts::default()
    };
    let query = tree(
        vec![dimension(0, "customers.region")],
        vec![metric(0, "revenue")],
        parts,
    );
    assert_eq!(
        bind_error(&query, &model, ANALYST),
        (BindCode::NameNotFound, ENTRY.to_owned())
    );
}
