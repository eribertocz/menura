//! Canonical JSON, hashing, value encoding and ordering: CON-2, CON-4 and CON-5.

use std::cmp::Ordering;

use menura_contracts::value_encoding::{
    is_canonical_date, is_canonical_decimal, is_canonical_integer_string, is_canonical_time,
    is_canonical_timestamp, is_canonical_uuid,
};
use menura_contracts::{canonical_json, compare_names, encode_integer, hash_value};
use serde_json::{Value, json};

/// The canonical form of `text`, or `None` when it is not JSON.
fn canonical(text: &str) -> Option<String> {
    serde_json::from_str::<Value>(text)
        .ok()
        .map(|value| canonical_json(&value))
}

/// RFC 8785 §3.2.4: the sample input and its canonical form.
#[test]
fn con_2_rfc_8785_sample() {
    let input = r#"{
        "numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],
        "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
        "literals": [null, true, false]
    }"#;
    assert_eq!(
        canonical(input).as_deref(),
        Some(
            r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#
        )
    );
}

/// RFC 8785 §3.2.3: members are sorted by UTF-16 code units.
#[test]
fn con_2_rfc_8785_member_sorting() {
    let input = r#"{
        "\u20ac": "Euro Sign",
        "\r": "Carriage Return",
        "\ufb33": "Hebrew Letter Dalet With Dagesh",
        "1": "One",
        "\ud83d\ude00": "Emoji: Grinning Face",
        "\u0080": "Control",
        "\u00f6": "Latin Small Letter O With Diaeresis"
    }"#;
    let expected = "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\u{80}\":\"Control\",\
        \"\u{f6}\":\"Latin Small Letter O With Diaeresis\",\"\u{20ac}\":\"Euro Sign\",\
        \"\u{1f600}\":\"Emoji: Grinning Face\",\"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}";
    assert_eq!(canonical(input).as_deref(), Some(expected));
}

#[test]
fn con_2_hash_is_sha256_of_canonical_json_in_lowercase_hex() {
    // SHA-256 of the two bytes `{}`.
    assert_eq!(
        hash_value(&json!({})).as_str(),
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
    // Member order and insignificant number forms do not change the hash.
    let left: Value = serde_json::from_str(r#"{"b": 1.0, "a": [10, "x"]}"#).expect("JSON");
    let right: Value = serde_json::from_str(r#"{"a": [1e1, "x"], "b": 1}"#).expect("JSON");
    assert_eq!(hash_value(&left), hash_value(&right));
    let hash = hash_value(&left);
    assert_eq!(hash.as_str().len(), 64);
    assert!(
        hash.as_str()
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
}

/// Integers beyond 2^53 collide as JSON numbers, so CON-4 writes them as strings, which hash
/// differently from their neighbors.
#[test]
fn con_2_integers_beyond_2_53_as_strings_hash_differently_from_their_neighbors() {
    let as_numbers: Vec<Value> =
        serde_json::from_str("[9007199254740992, 9007199254740993]").expect("JSON");
    assert_eq!(hash_value(&as_numbers[0]), hash_value(&as_numbers[1]));
    let below = hash_value(&encode_integer(9_007_199_254_740_992));
    let value = hash_value(&encode_integer(9_007_199_254_740_993));
    let above = hash_value(&encode_integer(9_007_199_254_740_994));
    assert_ne!(value, below);
    assert_ne!(value, above);
    assert_ne!(below, above);
}

#[test]
fn con_4_integers_are_numbers_only_within_the_safe_range() {
    assert_eq!(
        encode_integer(9_007_199_254_740_991),
        json!(9_007_199_254_740_991_i64)
    );
    assert_eq!(
        encode_integer(-9_007_199_254_740_991),
        json!(-9_007_199_254_740_991_i64)
    );
    assert_eq!(
        encode_integer(9_007_199_254_740_992),
        json!("9007199254740992")
    );
    assert_eq!(
        encode_integer(-9_007_199_254_740_992),
        json!("-9007199254740992")
    );
    assert!(is_canonical_integer_string("9007199254740992"));
    assert!(is_canonical_integer_string(
        "-170141183460469231731687303715884105728"
    ));
    assert!(!is_canonical_integer_string("9007199254740991"));
    assert!(!is_canonical_integer_string("09007199254740992"));
    assert!(!is_canonical_integer_string("+9007199254740992"));
}

#[test]
fn con_4_decimals_have_exactly_scale_digits() {
    assert!(is_canonical_decimal("-12.50", 2));
    assert!(is_canonical_decimal("0.00", 2));
    assert!(is_canonical_decimal("12", 0));
    assert!(!is_canonical_decimal("-12.5", 2));
    assert!(!is_canonical_decimal("012.50", 2));
    assert!(!is_canonical_decimal("-0.00", 2));
    assert!(!is_canonical_decimal("12.", 0));
    assert!(!is_canonical_decimal("1e2", 0));
}

#[test]
fn con_4_dates_times_timestamps_and_uuids() {
    assert!(is_canonical_date("2024-02-29"));
    assert!(!is_canonical_date("2023-02-29"));
    assert!(is_canonical_time("23:59:59"));
    assert!(is_canonical_time("12:00:00.5"));
    assert!(is_canonical_time("12:00:00.123456789"));
    assert!(!is_canonical_time("12:00:00.50"));
    assert!(!is_canonical_time("12:00:00.0"));
    assert!(!is_canonical_time("12:00:00.1234567891"));
    assert!(!is_canonical_time("24:00:00"));
    assert!(is_canonical_timestamp("2026-11-15T16:00:00Z", true));
    assert!(is_canonical_timestamp("2026-11-15T16:00:00.25", false));
    assert!(!is_canonical_timestamp("2026-11-15T16:00:00", true));
    assert!(!is_canonical_timestamp("2026-11-15T16:00:00Z", false));
    assert!(!is_canonical_timestamp("2026-11-15T16:00:00+00:00", true));
    assert!(is_canonical_uuid("123e4567-e89b-12d3-a456-426614174000"));
    assert!(!is_canonical_uuid("123E4567-E89B-12D3-A456-426614174000"));
    assert!(!is_canonical_uuid("123e4567e89b12d3a456426614174000"));
}

#[test]
fn con_5_names_are_ordered_by_code_point() {
    assert_eq!(compare_names("\u{FF61}", "\u{1F600}"), Ordering::Less);
    assert_eq!(compare_names("B", "a"), Ordering::Less);
    assert_eq!(compare_names("AB", "A"), Ordering::Greater);
    let mut names = vec!["\u{1F600}", "Z", "\u{FF61}", "A"];
    names.sort_by(|left, right| compare_names(left, right));
    assert_eq!(names, ["A", "Z", "\u{FF61}", "\u{1F600}"]);
}
