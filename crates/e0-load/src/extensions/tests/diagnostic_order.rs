//! The location and document order of extension diagnostics: E0C-2.6 and E0-4.3.

use super::{MODEL, pair, validate};
use crate::extensions::DEFAULT_NAMESPACE;

#[test]
fn e0c_2_6_e0_4_3_diagnostics_are_located_and_follow_document_order() {
    let document = concat!(
        "version: \"0.1.1\"\n",
        "semantic_model:\n",
        "  - name: shop\n",
        "    datasets:\n",
        "      - name: orders\n",
        "        source: shop.orders\n",
        "        custom_extensions:\n",
        "          - {vendor_name: COMMON, data: '{\"namespace\": \"menura\", \"version\": 1, \"owner\": 1}'}\n",
        "          - {vendor_name: COMMON, data: '{\"namespace\": \"menura\", \"version\": 2}'}\n",
        "          - {vendor_name: COMMON, data: 'broken'}\n",
        "    custom_extensions:\n",
        "      - {vendor_name: COMMON, data: '{\"namespace\": \"menura\"}'}\n",
    );
    let failure = validate(document.as_bytes(), DEFAULT_NAMESPACE).expect_err("rejected");
    let found: Vec<_> = failure
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect();
    let orders = format!("{MODEL}/datasets/0/custom_extensions");
    assert_eq!(
        found,
        [
            pair("E0C-02", &format!("{orders}/0/data")),
            pair("E0C-04", &format!("{orders}/1")),
            pair("E0C-01", &format!("{orders}/1/data")),
            pair("E0C-01", &format!("{MODEL}/custom_extensions/0/data")),
        ]
    );
    assert_eq!(failure.warnings.len(), 1);
    assert_eq!(
        failure.warnings[0].pointer.to_string(),
        format!("{orders}/2/data")
    );
}
