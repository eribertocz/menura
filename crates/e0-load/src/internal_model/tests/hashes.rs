//! E0H-1.3, E0H-2, E0H-2.1, E0H-2.2, E0H-3.1 to E0H-3.4 and MDL-3.1: what the model hash and
//! the summary definition hashes cover.

use semantic_compiler_model::internal_model::{DiagnosticCode, InternalModel, SourceLocation};
use serde_json::Value as Json;

use super::{
    CATALOG, MODEL, SUMMARIES_CATALOG, SUMMARIES_MODEL, edited, fixture, hash_of, load, name,
};
use crate::internal_model::hashing::model_hash;

/// `catalog` with the logical type of the column `column` of `source` widened to 64 bits.
pub(super) fn widened(catalog: &str, source: &str, column: &str) -> String {
    let mut document: Json = serde_json::from_str(catalog).expect("the catalog is JSON");
    for relation in document["relations"].as_array_mut().expect("relations") {
        if relation["source"] != source {
            continue;
        }
        for item in relation["columns"].as_array_mut().expect("columns") {
            if item["name"] == column {
                item["logical_type"]["bits"] = Json::from(64);
                item["physical_type"] = Json::from("BIGINT");
            }
        }
    }
    serde_json::to_string(&document).expect("the catalog is written")
}

fn summary_hash(model: &InternalModel) -> String {
    model.parts().summaries[0]
        .definition_hash
        .as_str()
        .to_owned()
}

#[test]
fn e0h_3_1_the_model_hash_is_64_lowercase_hex_digits() {
    let model = fixture();
    let hash = model.hash().as_str();
    assert_eq!(hash.len(), 64);
    assert!(
        hash.chars()
            .all(|digit| matches!(digit, '0'..='9' | 'a'..='f'))
    );
}

#[test]
fn mdl_3_1_the_model_hash_is_the_hash_of_e0h_3_2() {
    let model = fixture();
    assert_eq!(model_hash(model.parts()), *model.hash());
}

#[test]
fn e0h_1_3_format_key_order_keyword_case_parentheses_and_quoting_keep_the_hash() {
    let original = hash_of(MODEL);
    let reformatted = [
        edited("'SUM(orders.amount)'", "'sum( ( \"ORDERS\".amount ) )'"),
        edited(
            "{dialect: ANSI_SQL, expression: 'order_id'}",
            "{expression: 'order_id', dialect: ANSI_SQL}",
        ),
        edited(
            "        from_columns: [customer_id]\n",
            "        from_columns:\n          - customer_id\n",
        ),
    ];
    for model in reformatted {
        assert_eq!(hash_of(&model), original, "{model}");
    }
}

#[test]
fn e0h_1_3_renaming_a_field_or_changing_a_literal_scale_changes_the_hash() {
    assert_ne!(
        hash_of(&edited("- name: region", "- name: area")),
        hash_of(MODEL)
    );
    let scaled = |literal: &str| {
        hash_of(&edited(
            "'SUM(orders.amount)'",
            &format!("'SUM(orders.amount * {literal})'"),
        ))
    };
    assert_ne!(scaled("1.5"), scaled("1.50"));
}

#[test]
fn e0h_2_a_description_is_part_of_the_model_hash() {
    assert_ne!(
        hash_of(&edited("'Order amount'", "'Amount of the order'")),
        hash_of(MODEL)
    );
}

#[test]
fn e0h_2_1_other_dialects_and_foreign_extensions_never_change_the_hash() {
    let other_dialect = edited(
        "{dialect: ANSI_SQL, expression: 'amount'}",
        "{dialect: ANSI_SQL, expression: 'amount'}, {dialect: SNOWFLAKE, expression: 'amount * 2'}",
    );
    let foreign_extension = edited(
        "        custom_extensions:\n          - vendor_name: COMMON\n            data: '{\"namespace\": \"semantic_compiler\", \"version\": 1, \"access\"",
        "        custom_extensions:\n          - vendor_name: DBT\n            data: '{\"materialized\": \"table\"}'\n          - vendor_name: COMMON\n            data: '{\"namespace\": \"semantic_compiler\", \"version\": 1, \"access\"",
    );
    assert_eq!(hash_of(&other_dialect), hash_of(MODEL));
    assert_eq!(hash_of(&foreign_extension), hash_of(MODEL));
}

#[test]
fn e0h_2_2_a_type_change_in_the_catalog_schema_changes_the_hash() {
    let catalog = widened(CATALOG, "shop.main.orders", "amount");
    assert_ne!(load(MODEL, &catalog).hash().as_str(), hash_of(MODEL));
}

#[test]
fn e0h_3_2_unusable_elements_and_their_codes_are_part_of_the_model_hash() {
    let unparsable = edited("'no_such_column'", "'no_such_column +'");
    let model = load(&unparsable, CATALOG);
    let discount = model
        .dataset(&name("orders"))
        .and_then(|dataset| dataset.field(&name("discount")))
        .expect("the field is in the model");
    assert_eq!(discount.query_errors, vec![DiagnosticCode::new("E0F-01")]);
    assert_ne!(model.hash().as_str(), hash_of(MODEL));
}

#[test]
fn e0h_3_4_warnings_and_source_locations_change_no_hash() {
    let model = fixture();
    let mut parts = model.parts().clone();
    let elsewhere = SourceLocation {
        pointer: "/elsewhere".to_owned(),
        line: 99,
        column: 9,
    };
    for warning in &mut parts.warnings {
        warning.message = "another wording".to_owned();
        warning.location = elsewhere.clone();
    }
    for element in &mut parts.unusable {
        element.load_warning = Some(DiagnosticCode::new("W0X-99"));
        element.location = elsewhere.clone();
    }
    for dataset in &mut parts.datasets {
        dataset.location = elsewhere.clone();
    }
    assert!(!parts.warnings.is_empty());
    assert_eq!(model_hash(&parts), *model.hash());
}

#[test]
fn e0h_3_4_moving_the_text_keeps_every_hash() {
    let moved = format!("# A comment line.\n\n{MODEL}");
    let original = fixture();
    let model = load(&moved, CATALOG);
    assert_ne!(
        model.parts().datasets[0].location,
        original.parts().datasets[0].location
    );
    assert_eq!(model.hash(), original.hash());
    assert_eq!(model.parts().checks, original.parts().checks);
}

#[test]
fn e0h_3_3_a_summary_hash_follows_the_fields_it_refers_to() {
    let original = load(SUMMARIES_MODEL, SUMMARIES_CATALOG);
    let expression = SUMMARIES_MODEL.replacen("'amount'", "'amount + 1'", 1);
    let catalog = widened(SUMMARIES_CATALOG, "shop.main.orders", "amount");
    assert_ne!(
        summary_hash(&load(&expression, SUMMARIES_CATALOG)),
        summary_hash(&original)
    );
    assert_ne!(
        summary_hash(&load(SUMMARIES_MODEL, &catalog)),
        summary_hash(&original)
    );
}

#[test]
fn e0h_3_3_a_summary_hash_ignores_an_unrelated_metric() {
    let original = load(SUMMARIES_MODEL, SUMMARIES_CATALOG);
    let unrelated =
        SUMMARIES_MODEL.replacen("'COUNT(orders.order_id)'", "'MAX(orders.order_id)'", 1);
    let model = load(&unrelated, SUMMARIES_CATALOG);
    assert_eq!(summary_hash(&model), summary_hash(&original));
    assert_ne!(model.hash(), original.hash());
}
