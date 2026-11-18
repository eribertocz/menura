//! The additivity extension in E0.c: ADD-2.2, ADD-3, ADD-5.1.1 to ADD-5.1.4, `E0C-02`,
//! `E0C-03`, `E0C-10`, `E0C-11` and `E0C-13` to `E0C-15`.
//!
//! The model documents of the `e0-additivity*` conformance cases are read here at compile time
//! (see [`super::fixture`]), so the rules are checked against the files the conformance suite
//! uses.

use std::num::NonZeroU64;

use semantic_compiler_contracts::types::audit_record::OssieFormat;
use semantic_compiler_contracts::types::common::Grain;

use super::fixture::{
    DEBT_BALANCE, DEBT_BALANCE_DATA, MODEL, STRICT_BALANCE, conformance_model, with_additivity,
    with_example,
};
use super::model::{
    DeclaredAdditivity, MissingHandling, NullHandling, Selection, SnapshotAggregation,
    StaleHandling, UnexpectedHandling,
};
use crate::extensions::output::{Extensions, ExtensionsFailure};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::names::Name;
use crate::read::read_document;
use crate::structure::validate_structure;

fn validate(model: &str) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

/// The codes of the rejecting diagnostics, each at the `data` member of `debt_balance`.
fn codes(model: &str) -> Vec<&'static str> {
    let failure = validate(model).expect_err("the extensions are rejected");
    failure
        .errors
        .iter()
        .map(|diagnostic| {
            assert_eq!(diagnostic.pointer.as_str(), DEBT_BALANCE_DATA);
            diagnostic.code.as_str()
        })
        .collect()
}

/// The codes found when `debt_balance` declares `additivity`.
fn declared(additivity: &str) -> Vec<&'static str> {
    codes(&with_additivity(additivity))
}

/// The codes found when the example of ADD-8 is changed by `replacements`.
fn example(replacements: &[(&str, &str)]) -> Vec<&'static str> {
    codes(&with_example(replacements))
}

/// Whether the extensions of `model` are accepted.
fn accepted(model: &str) -> bool {
    validate(model).is_ok()
}

#[test]
fn add_3_add_8_the_example_is_read() {
    let extensions = validate(MODEL).expect("the extensions are accepted");
    let [semi, non_additive] = extensions.additivity.as_slice() else {
        panic!("two declarations, found {:?}", extensions.additivity);
    };
    assert_eq!(semi.metric.as_str(), DEBT_BALANCE);
    assert_eq!(semi.data.as_str(), DEBT_BALANCE_DATA);
    assert_eq!(non_additive.metric.as_str(), STRICT_BALANCE);
    assert_eq!(non_additive.additivity, DeclaredAdditivity::NonAdditive);
    let keys = semi.additivity.semi_additive().expect("semi-additive");
    assert_eq!(keys.over.dataset, Name::regular("debt"));
    assert_eq!(keys.over.field, Name::regular("period"));
    let entity: Vec<&str> = keys
        .entity
        .iter()
        .map(|field| field.written.as_str())
        .collect();
    assert_eq!(entity, ["debt.customer_id"]);
    assert_eq!(
        keys.aggregation,
        SnapshotAggregation::Last(Selection::GlobalPeriod)
    );
    assert_eq!(keys.nulls, NullHandling::AsMissing);
    assert_eq!(keys.snapshot_grain, Some(Grain::Month));
    assert_eq!(keys.population.as_deref(), Some("active_customers"));
    assert_eq!(
        keys.missing,
        Some(MissingHandling::CarryForward {
            max_carry_forward: NonZeroU64::new(2).expect("not zero"),
            on_stale: StaleHandling::Error,
        })
    );
    assert_eq!(keys.unexpected, Some(UnexpectedHandling::Exclude));
}

#[test]
fn add_3_2_optional_keys_may_be_left_out() {
    let minimal = r#"{"type": "semi_additive", "over": "debt.period", "entity": ["debt.customer_id"], "aggregation": "min", "nulls": "keep"}"#;
    let extensions = validate(&with_additivity(minimal)).expect("the extensions are accepted");
    let keys = extensions.additivity[0]
        .additivity
        .semi_additive()
        .expect("semi-additive")
        .clone();
    assert_eq!(keys.aggregation, SnapshotAggregation::Minimum);
    assert_eq!(
        (
            keys.snapshot_grain,
            keys.population,
            keys.missing,
            keys.unexpected
        ),
        (None, None, None, None)
    );
}

#[test]
fn add_5_1_1_e0c_10_non_additive_admits_no_other_key() {
    let model = conformance_model("e0-additivity-non-additive-key-001");
    let failure = validate(model).expect_err("the extensions are rejected");
    let found: Vec<&str> = failure
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert_eq!(found, ["E0C-10"]);
    assert_eq!(
        declared(r#"{"type": "non_additive", "colour": 1}"#),
        ["E0C-10"]
    );
    assert_eq!(
        declared(r#"{"type": "non_additive", "nulls": "keep", "colour": 1}"#),
        ["E0C-10"]
    );
}

#[test]
fn add_5_1_1_e0c_11_required_keys() {
    let model = conformance_model("e0-additivity-missing-key-001");
    let failure = validate(model).expect_err("the extensions are rejected");
    let found: Vec<&str> = failure
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert_eq!(found, ["E0C-11"]);
    assert_eq!(declared("{}"), ["E0C-11"]);
    assert_eq!(declared(r#"{"over": "debt.period"}"#), ["E0C-11"]);
    for key in [
        r#""over": "debt.period", "#,
        r#""entity": ["debt.customer_id"], "#,
        r#", "nulls": "as_missing""#,
    ] {
        assert_eq!(example(&[(key, "")]), ["E0C-11"], "{key}");
    }
    assert_eq!(
        example(&[(
            r#""aggregation": "last", "selection": "global_period", "#,
            ""
        )]),
        ["E0C-11"]
    );
}

#[test]
fn add_5_1_1_e0c_02_a_key_no_type_defines() {
    assert_eq!(
        example(&[(r#""nulls""#, r#""colour": 1, "nulls""#)]),
        ["E0C-02"]
    );
    assert_eq!(declared(r#"{"colour": 1}"#), ["E0C-02", "E0C-11"]);
    assert_eq!(
        declared(r#"{"type": "additive", "colour": 1}"#),
        ["E0C-02", "E0C-03"]
    );
}

#[test]
fn add_2_2_add_5_1_2_e0c_03_values() {
    assert_eq!(declared(r#"{"type": "additive"}"#), ["E0C-03"]);
    assert_eq!(declared(r#""non_additive""#), ["E0C-03"]);
    for (from, to) in [
        (r#""aggregation": "last""#, r#""aggregation": "sum""#),
        (r#""nulls": "as_missing""#, r#""nulls": null"#),
        (
            r#""snapshot_grain": "month""#,
            r#""snapshot_grain": "hour""#,
        ),
        (r#""population": "active_customers""#, r#""population": 1"#),
        (r#""missing": "carry_forward""#, r#""missing": "skip""#),
        (r#""unexpected": "exclude""#, r#""unexpected": "keep""#),
        (
            r#""selection": "global_period""#,
            r#""selection": "latest""#,
        ),
        (r#""on_stale": "error""#, r#""on_stale": "zero""#),
        (r#""max_carry_forward": 2"#, r#""max_carry_forward": 2.0"#),
        (r#""max_carry_forward": 2"#, r#""max_carry_forward": "2""#),
        (r#""entity": ["debt.customer_id"]"#, r#""entity": []"#),
        (
            r#""entity": ["debt.customer_id"]"#,
            r#""entity": "debt.customer_id""#,
        ),
        (
            r#""entity": ["debt.customer_id"]"#,
            r#""entity": ["debt.customer_id", "DEBT.CUSTOMER_ID"]"#,
        ),
        (r#""over": "debt.period""#, r#""over": ["debt.period"]"#),
    ] {
        assert_eq!(example(&[(from, to)]), ["E0C-03"], "{to}");
    }
}

#[test]
fn add_5_1_3_e0c_03_field_references_are_qualified() {
    for to in [
        r#""over": "period""#,
        r#""over": "debt.period + 1""#,
        r#""over": "debt.\"period""#,
    ] {
        assert_eq!(
            example(&[(r#""over": "debt.period""#, to)]),
            ["E0C-03"],
            "{to}"
        );
    }
    assert_eq!(
        example(&[(r#"["debt.customer_id"]"#, r#"["customer_id"]"#)]),
        ["E0C-03"]
    );
    assert!(accepted(&with_example(&[(
        r#""over": "debt.period""#,
        r#""over": "\"DEBT\".\"PERIOD\"""#
    )])));
}

#[test]
fn add_5_1_2_add_5_1_4_e0c_13_carry_forward_keys() {
    let model = conformance_model("e0-additivity-carry-forward-001");
    let failure = validate(model).expect_err("the extensions are rejected");
    let found: Vec<&str> = failure
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert_eq!(found, ["E0C-13"]);
    for replacements in [
        [(r#""max_carry_forward": 2"#, r#""max_carry_forward": -1"#)],
        [(r#""missing": "carry_forward""#, r#""missing": "exclude""#)],
        [(r#""max_carry_forward": 2, "#, "")],
        [(r#""on_stale": "error", "#, "")],
    ] {
        assert_eq!(example(&replacements), ["E0C-13"], "{replacements:?}");
    }
    assert_eq!(
        example(&[(r#""missing": "carry_forward", "#, "")]),
        ["E0C-13"]
    );
    assert!(accepted(&with_example(&[
        (r#""missing": "carry_forward""#, r#""missing": "zero""#),
        (r#""max_carry_forward": 2, "#, ""),
        (r#""on_stale": "error", "#, ""),
    ])));
}

#[test]
fn add_5_1_4_e0c_14_selection_with_first_or_last() {
    let model = conformance_model("e0-additivity-selection-001");
    let failure = validate(model).expect_err("the extensions are rejected");
    let found: Vec<&str> = failure
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert_eq!(found, ["E0C-14"]);
    assert_eq!(
        example(&[(r#""aggregation": "last""#, r#""aggregation": "max""#)]),
        ["E0C-14"]
    );
    assert!(accepted(&with_example(&[
        (r#""aggregation": "last""#, r#""aggregation": "first""#),
        (r#""global_period""#, r#""per_entity""#),
    ])));
    assert!(accepted(&with_example(&[
        (r#""aggregation": "last""#, r#""aggregation": "avg""#),
        (r#""selection": "global_period", "#, ""),
    ])));
}

#[test]
fn add_5_1_3_e0c_15_over_is_not_part_of_entity() {
    let model = conformance_model("e0-additivity-over-in-entity-001");
    let failure = validate(model).expect_err("the extensions are rejected");
    let found: Vec<&str> = failure
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert_eq!(found, ["E0C-15"]);
    assert_eq!(
        example(&[(r#"["debt.customer_id"]"#, r#"["\"DEBT\".period"]"#)]),
        ["E0C-15"]
    );
}

#[test]
fn add_5_1_one_diagnostic_per_code_in_the_order_of_the_codes() {
    let failure = validate(&with_example(&[
        (
            r#""nulls": "as_missing""#,
            r#""nulls": "none", "colour": 1"#,
        ),
        (r#""aggregation": "last""#, r#""aggregation": "min""#),
        (r#""max_carry_forward": 2"#, r#""max_carry_forward": 0"#),
        (r#""population": "active_customers""#, r#""population": 1"#),
    ]))
    .expect_err("the extensions are rejected");
    let found: Vec<&str> = failure
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert_eq!(found, ["E0C-02", "E0C-03", "E0C-13", "E0C-14"]);
    assert!(failure.errors[1].message.contains("`nulls`"));
    assert!(failure.errors[1].message.contains("`population`"));
}

#[test]
fn add_5_the_checks_of_later_substages_are_not_made_in_e0c() {
    for model in [
        "e0-additivity-time-kind-001",
        "e0-additivity-unreachable-001",
        "e0-additivity-population-001",
        "e0-additivity-conflicts",
    ] {
        assert!(accepted(conformance_model(model)), "{model}");
    }
}
