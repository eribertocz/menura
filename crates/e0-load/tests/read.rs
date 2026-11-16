//! Reading the model document: E0A-2.1 to E0A-2.8, E0A-01 to E0A-05, MDL-1.2 and DEV-1.
//!
//! The model documents of the `e0` conformance cases are read here at compile time, so the
//! reader is checked against the same files the conformance suite uses.

use semantic_compiler_e0_load::read::{Location, Node, ReadErrorCode, Value, read_document};

// `cfg(test)` marks the helpers as test code, where `expect` and `panic` are allowed (clippy.toml).

#[cfg(test)]
fn read(text: &str) -> Node {
    read_document(text.as_bytes()).expect("the document is read")
}

/// The code and location of the error of a document that is rejected.
#[cfg(test)]
fn failure(bytes: &[u8]) -> (ReadErrorCode, Location) {
    let error = read_document(bytes).expect_err("the document is rejected");
    (error.code, error.location)
}

#[cfg(test)]
fn at(line: usize, column: usize) -> Location {
    Location { line, column }
}

/// The value of `key` in a mapping node.
#[cfg(test)]
fn get<'node>(node: &'node Node, key: &str) -> &'node Node {
    let Value::Mapping(entries) = &node.value else {
        panic!("not a mapping: {node:?}");
    };
    entries
        .iter()
        .find(|entry| entry.key.value == Value::String(key.to_owned()))
        .map_or_else(|| panic!("no key {key}"), |entry| &entry.value)
}

/// The item `index` of a sequence node.
#[cfg(test)]
fn item(node: &Node, index: usize) -> &Node {
    let Value::Sequence(items) = &node.value else {
        panic!("not a sequence: {node:?}");
    };
    &items[index]
}

/// The keys of a mapping node, as text, in order.
#[cfg(test)]
fn keys(node: &Node) -> Vec<String> {
    let Value::Mapping(entries) = &node.value else {
        panic!("not a mapping: {node:?}");
    };
    entries
        .iter()
        .map(|entry| match &entry.key.value {
            Value::String(text) => text.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

#[cfg(test)]
fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

#[cfg(test)]
fn integer(node: &Node) -> String {
    let Value::Integer(value) = &node.value else {
        panic!("not an integer: {node:?}");
    };
    value.to_string()
}

#[cfg(test)]
fn float(node: &Node) -> f64 {
    let Value::Float(value) = node.value else {
        panic!("not a float: {node:?}");
    };
    value
}

/// The `fields` of the first dataset of the first semantic model of a model document.
#[cfg(test)]
fn first_dataset_fields(document: &Node) -> &Node {
    get(
        item(get(item(get(document, "semantic_model"), 0), "datasets"), 0),
        "fields",
    )
}

// E0A-2.1, E0A-01

#[test]
fn e0a_2_1_invalid_utf8_is_e0a_01() {
    let model = include_bytes!("../../../conformance/models/e0-read-invalid-utf8-001/model.yaml");
    assert_eq!(failure(model), (ReadErrorCode::InvalidEncoding, at(4, 59)));
}

#[test]
fn e0a_2_1_utf8_text_and_byte_order_mark_are_read() {
    let document = read("\u{feff}name: caf\u{e9}\n");
    assert_eq!(get(&document, "name").value, text("caf\u{e9}"));
}

// E0A-2.2, E0A-02

#[test]
fn e0a_2_2_yes_no_on_off_read_as_booleans() {
    let document = read("a: yes\nb: No\nc: ON\nd: off\ne: true\nf: FALSE\ng: 'yes'\nh: y\n");
    let booleans: Vec<Value> = ["a", "b", "c", "d", "e", "f", "g", "h"]
        .iter()
        .map(|key| get(&document, key).value.clone())
        .collect();
    assert_eq!(
        booleans,
        vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Boolean(true),
            Value::Boolean(false),
            text("yes"),
            text("y"),
        ]
    );
}

#[test]
fn e0a_2_2_yes_case_model_reads_description_as_boolean() {
    let model = include_bytes!("../../../conformance/models/e0-read-yes-boolean-001/model.yaml");
    let document = read_document(model).expect("the document is read");
    let description = get(item(get(&document, "semantic_model"), 0), "description");
    assert_eq!(description.value, Value::Boolean(true));
}

#[test]
fn e0a_2_2_yaml_1_1_integers() {
    let document = read(
        "decimal: 1_000\nnegative: -42\noctal: 012\nhexadecimal: 0xFF\nbinary: 0b101\n\
         sexagesimal: 1:30\nzero: -0\nlarge: 123456789012345678901234567890\nnot_octal: 08\n",
    );
    let read_integer = |key| integer(get(&document, key));
    assert_eq!(read_integer("decimal"), "1000");
    assert_eq!(read_integer("negative"), "-42");
    assert_eq!(read_integer("octal"), "10");
    assert_eq!(read_integer("hexadecimal"), "255");
    assert_eq!(read_integer("binary"), "5");
    assert_eq!(read_integer("sexagesimal"), "90");
    assert_eq!(read_integer("zero"), "0");
    assert_eq!(read_integer("large"), "123456789012345678901234567890");
    assert_eq!(get(&document, "not_octal").value, text("08"));
}

#[test]
fn e0a_2_2_yaml_1_1_floats() {
    let document = read(
        "plain: 1.5\nexponent: 1.0e+3\nleading_dot: .5\nsexagesimal: 1:30.5\n\
         infinity: -.inf\nnot_a_number: .NaN\nno_dot: 1e3\n",
    );
    assert!((float(get(&document, "plain")) - 1.5).abs() < f64::EPSILON);
    assert!((float(get(&document, "exponent")) - 1000.0).abs() < f64::EPSILON);
    assert!((float(get(&document, "leading_dot")) - 0.5).abs() < f64::EPSILON);
    assert!((float(get(&document, "sexagesimal")) - 90.5).abs() < f64::EPSILON);
    let infinity = float(get(&document, "infinity"));
    assert!(infinity.is_infinite() && infinity.is_sign_negative());
    assert!(float(get(&document, "not_a_number")).is_nan());
    assert_eq!(get(&document, "no_dot").value, text("1e3"));
}

#[test]
fn e0a_2_2_nulls_and_strings() {
    let document = read("a: ~\nb: null\nc:\nd: 'null'\ne: |\n  block\n");
    assert_eq!(get(&document, "a").value, Value::Null);
    assert_eq!(get(&document, "b").value, Value::Null);
    assert_eq!(get(&document, "c").value, Value::Null);
    assert_eq!(get(&document, "d").value, text("null"));
    assert_eq!(get(&document, "e").value, text("block\n"));
}

#[test]
fn e0a_2_2_standard_tags_are_constructed() {
    let document = read(
        "a: !!str 12\nb: !!int '12'\nc: !!float 1\nd: !!bool yEs\ne: !!null x\nf: ! 12\n\
         g: !!seq [1]\nh: !!map {k: v}\ni: !!timestamp 2026-1-5\n",
    );
    assert_eq!(get(&document, "a").value, text("12"));
    assert_eq!(integer(get(&document, "b")), "12");
    assert!((float(get(&document, "c")) - 1.0).abs() < f64::EPSILON);
    assert_eq!(get(&document, "d").value, Value::Boolean(true));
    assert_eq!(get(&document, "e").value, Value::Null);
    assert_eq!(get(&document, "f").value, text("12"));
    assert_eq!(integer(item(get(&document, "g"), 0)), "1");
    assert_eq!(get(get(&document, "h"), "k").value, text("v"));
    assert_eq!(get(&document, "i").value, text("2026-01-05"));
}

#[test]
fn e0a_2_2_json_document_reads_as_yaml() {
    let document = read(r#"{"version": "0.1.1", "list": [1, 2.5, true, null, "x"]}"#);
    assert_eq!(get(&document, "version").value, text("0.1.1"));
    let list = get(&document, "list");
    assert_eq!(integer(item(list, 0)), "1");
    assert!((float(item(list, 1)) - 2.5).abs() < f64::EPSILON);
    assert_eq!(item(list, 2).value, Value::Boolean(true));
    assert_eq!(item(list, 3).value, Value::Null);
    assert_eq!(item(list, 4).value, text("x"));
}

#[test]
fn e0a_2_2_tag_outside_the_standard_tags_is_e0a_05() {
    let model = include_bytes!("../../../conformance/models/e0-read-unknown-tag-001/model.yaml");
    assert_eq!(failure(model), (ReadErrorCode::Unconstructible, at(4, 18)));
    assert_eq!(
        failure(b"a: !local value\n"),
        (ReadErrorCode::Unconstructible, at(1, 4))
    );
}

#[test]
fn e0a_2_2_value_key_is_text_and_value_scalar_is_e0a_05() {
    assert_eq!(get(&read("=: a\n"), "=").value, text("a"));
    assert_eq!(
        failure(b"a: =\n"),
        (ReadErrorCode::Unconstructible, at(1, 4))
    );
}

#[test]
fn e0a_2_2_unparsable_yaml_is_e0a_02() {
    let model = include_bytes!("../../../conformance/models/e0-read-unparsable-001/model.yaml");
    assert_eq!(failure(model).0, ReadErrorCode::Unparsable);
    assert_eq!(failure(b"a: *unknown\n").0, ReadErrorCode::Unparsable);
}

// E0A-2.3, E0A-04, DEV-1

#[test]
fn e0a_2_3_duplicate_key_in_nested_mapping_is_e0a_04() {
    let model = include_bytes!("../../../conformance/models/e0-read-duplicate-key-001/model.yaml");
    assert_eq!(failure(model), (ReadErrorCode::DuplicateKey, at(17, 13)));
}

#[test]
fn dev_1_duplicate_top_level_key_is_e0a_04() {
    assert_eq!(
        failure(b"a: 1\nb: 2\na: 3\n"),
        (ReadErrorCode::DuplicateKey, at(3, 1))
    );
}

#[test]
fn e0a_2_3_keys_equal_after_construction_are_duplicates() {
    assert_eq!(failure(b"{1: a, 0x1: b}").0, ReadErrorCode::DuplicateKey);
    assert_eq!(failure(b"{yes: a, true: b}").0, ReadErrorCode::DuplicateKey);
    assert_eq!(failure(b"{1: a, 1.0: b}").0, ReadErrorCode::DuplicateKey);
    assert_eq!(failure(b"{true: a, 1: b}").0, ReadErrorCode::DuplicateKey);
    assert_eq!(
        failure(b"{2026-11-16: a, '2026-11-16': b}").0,
        ReadErrorCode::DuplicateKey
    );
    assert_eq!(keys(&read("{1: a, '1': b}")).len(), 2);
}

#[test]
fn e0a_2_3_repeated_merge_key_is_e0a_04() {
    assert_eq!(
        failure(b"a: {x: 1}\nb:\n  <<: {y: 1}\n  <<: {z: 1}\n"),
        (ReadErrorCode::DuplicateKey, at(4, 3))
    );
}

#[test]
fn e0a_2_3_collection_key_is_e0a_05() {
    assert_eq!(
        failure(b"? [a]\n: b\n"),
        (ReadErrorCode::Unconstructible, at(1, 3))
    );
}

// E0A-2.4

#[test]
fn e0a_2_4_merge_key_case_expands() {
    let model = include_bytes!("../../../conformance/models/e0-read-merge-key-001/model.yaml");
    let document = read_document(model).expect("the document is read");
    let fields = first_dataset_fields(&document);
    let names: Vec<&Value> = (0..3)
        .map(|index| &get(item(fields, index), "name").value)
        .collect();
    assert_eq!(
        names,
        vec![&text("order_id"), &text("status"), &text("amount")]
    );
    let expression = |index| {
        &get(
            item(get(get(item(fields, index), "expression"), "dialects"), 0),
            "expression",
        )
        .value
    };
    assert_eq!(expression(1), &text("status"));
    assert_eq!(expression(2), &text("amount"));
}

#[test]
fn e0a_2_4_explicit_keys_override_merged_keys_and_first_merge_wins() {
    let document = read(
        "base: &base {a: 1, b: 1}\nother: &other {b: 2, c: 2}\n\
         merged:\n  <<: [*other, *base]\n  c: 3\n  d: 3\n",
    );
    let merged = get(&document, "merged");
    assert_eq!(keys(merged), vec!["a", "b", "c", "d"]);
    assert_eq!(integer(get(merged, "a")), "1");
    assert_eq!(integer(get(merged, "b")), "2");
    assert_eq!(integer(get(merged, "c")), "3");
}

#[test]
fn e0a_2_4_aliases_expand_to_the_anchored_node() {
    let document = read("a: &shared [1, {b: 2}]\nc: *shared\n");
    assert_eq!(get(&document, "a"), get(&document, "c"));
}

#[test]
fn e0a_2_4_merge_of_a_scalar_is_e0a_05() {
    assert_eq!(failure(b"a:\n  <<: 1\n").0, ReadErrorCode::Unconstructible);
    assert_eq!(
        failure(b"a:\n  <<: [{b: 1}, 2]\n").0,
        ReadErrorCode::Unconstructible
    );
}

#[test]
fn e0a_2_4_alias_inside_its_own_anchor_is_e0a_05() {
    assert_eq!(
        failure(b"a: &loop [*loop]\n").0,
        ReadErrorCode::Unconstructible
    );
}

// E0A-2.5, E0A-05

#[test]
fn e0a_2_5_tag_value_that_cannot_be_constructed_is_e0a_05() {
    let model =
        include_bytes!("../../../conformance/models/e0-read-unconstructible-value-001/model.yaml");
    assert_eq!(failure(model), (ReadErrorCode::Unconstructible, at(1, 10)));
    for document in [
        "a: !!bool maybe\n",
        "a: !!float x\n",
        "a: !!timestamp yesterday\n",
        "a: !!seq x\n",
        "a: !!str [x]\n",
        "a: 0b_\n",
    ] {
        assert_eq!(
            failure(document.as_bytes()).0,
            ReadErrorCode::Unconstructible,
            "{document}"
        );
    }
}

#[test]
fn e0a_2_5_standard_tags_without_a_json_form_are_e0a_05() {
    for document in [
        "a: !!binary aGVsbG8=\n",
        "a: !!set {x: ~}\n",
        "a: !!omap [{x: 1}]\n",
        "a: !!pairs [{x: 1}]\n",
    ] {
        assert_eq!(
            failure(document.as_bytes()),
            (ReadErrorCode::Unconstructible, at(1, 4)),
            "{document}"
        );
    }
}

#[test]
fn e0a_2_5_invalid_date_is_e0a_05() {
    let model = include_bytes!("../../../conformance/models/e0-read-invalid-date-001/model.yaml");
    assert_eq!(failure(model), (ReadErrorCode::Unconstructible, at(4, 18)));
    for document in [
        "a: 2025-02-29\n",
        "a: 0000-01-01\n",
        "a: 2026-11-16 24:00:00\n",
        "a: 2026-11-16 10:00:60\n",
        "a: 2026-11-16 10:00:00 +24\n",
    ] {
        assert_eq!(
            failure(document.as_bytes()).0,
            ReadErrorCode::Unconstructible,
            "{document}"
        );
    }
}

// E0A-2.6, E0A-03

#[test]
fn e0a_2_6_second_document_is_e0a_03() {
    let model = include_bytes!("../../../conformance/models/e0-read-two-documents-001/model.yaml");
    assert_eq!(failure(model), (ReadErrorCode::SeveralDocuments, at(43, 1)));
}

#[test]
fn e0a_2_6_explicit_single_document_and_empty_stream_are_read() {
    assert_eq!(get(&read("---\na: 1\n...\n"), "a").location, at(2, 4));
    assert_eq!(read("# only a comment\n").value, Value::Null);
}

// E0A-2.7

#[test]
fn e0a_2_7_every_node_keeps_line_and_column() {
    let document = read("a:\n  - x\n  - {b: 1}\n");
    assert_eq!(document.location, at(1, 1));
    let Value::Mapping(entries) = &document.value else {
        panic!("not a mapping");
    };
    assert_eq!(entries[0].key.location, at(1, 1));
    let list = get(&document, "a");
    assert_eq!(list.location, at(2, 3));
    assert_eq!(item(list, 0).location, at(2, 5));
    assert_eq!(item(list, 1).location, at(3, 5));
    assert_eq!(get(item(list, 1), "b").location, at(3, 9));
}

// E0A-2.8

#[test]
fn e0a_2_8_date_case_converts_to_iso_text() {
    let model = include_bytes!("../../../conformance/models/e0-read-date-text-001/model.yaml");
    let document = read_document(model).expect("the document is read");
    let description = get(item(get(&document, "semantic_model"), 0), "description");
    assert_eq!(description.value, text("2026-11-16"));
}

#[test]
fn e0a_2_8_timestamps_convert_to_iso_text() {
    let document = read(
        "a: 2026-11-16t10:20:30Z\nb: 2026-1-2 3:04:05.5 -4\nc: 2026-11-16T10:20:30.1234567+05:30\n\
         d: 2026-11-16 10:20:30\ne: 2026-11-16 10:20:30.000 -00:00\n",
    );
    assert_eq!(get(&document, "a").value, text("2026-11-16T10:20:30+00:00"));
    assert_eq!(
        get(&document, "b").value,
        text("2026-01-02T03:04:05.500000-04:00")
    );
    assert_eq!(
        get(&document, "c").value,
        text("2026-11-16T10:20:30.123456+05:30")
    );
    assert_eq!(get(&document, "d").value, text("2026-11-16T10:20:30"));
    assert_eq!(get(&document, "e").value, text("2026-11-16T10:20:30+00:00"));
}

// MDL-1.2

#[test]
fn mdl_1_2_sample_model_is_read() {
    let model = include_bytes!("../../../conformance/models/sample/model.yaml");
    let document = read_document(model).expect("the document is read");
    assert_eq!(get(&document, "version").value, text("0.1.1"));
    assert_eq!(keys(&document), vec!["version", "semantic_model"]);
}
