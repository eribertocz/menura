//! The load options of architecture section 3.5, as a JSON object (ARC-5.5).

use semantic_compiler_core::{LoadOptions, OssieFormat};

#[test]
fn arc_5_5_an_omitted_option_takes_its_default() {
    let options = LoadOptions::from_json(b"{}").unwrap();
    assert_eq!(options, LoadOptions::default());
    assert_eq!(options.ossie_format, OssieFormat::Release);
    assert_eq!(options.namespace, "semantic_compiler");
    assert!(!options.grant_authenticated_to_all);
    assert_eq!(options.model_name, None);
    assert_eq!(options.data_source, None);
}

#[test]
fn arc_5_5_every_option_is_read() {
    let options = LoadOptions::from_json(
        br#"{"model_name": "shop", "ossie_format": "development", "grant_authenticated_to_all": true,
             "namespace": "acme", "data_source": "warehouse"}"#,
    )
    .unwrap();
    assert_eq!(options.model_name.as_deref(), Some("shop"));
    assert_eq!(options.ossie_format, OssieFormat::Development);
    assert!(options.grant_authenticated_to_all);
    assert_eq!(options.namespace, "acme");
    assert_eq!(options.data_source.as_deref(), Some("warehouse"));
}

#[test]
fn arc_5_5_a_document_that_is_not_an_object_of_load_options_is_refused() {
    for text in [
        &b"[]"[..],
        b"not json",
        &[0xff],
        br#"{"dialect": "duckdb"}"#,
        br#"{"ossie_format": "draft"}"#,
        br#"{"grant_authenticated_to_all": "yes"}"#,
        br#"{"model_name": 1}"#,
        br#"{"model_name": null}"#,
    ] {
        assert!(LoadOptions::from_json(text).is_err(), "{text:?} is refused");
    }
}
