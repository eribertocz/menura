//! SEC-3.9: the former contract name of the security context (CON-0.5).

mod support;

use menura_contracts::types::compiled_plan::InputDocument;
use menura_security::admit_security_context;
use support::{document, document_named};

#[test]
fn sec_3_9_former_name_reports_wqry_01_at_contract() {
    let text = document_named("semantic_compiler.security_context", "u", "[]", "{}");
    let context = admit_security_context(&text, None).unwrap();
    let warning = context.former_name_warning().unwrap();
    assert_eq!(warning.code, "WQRY-01");
    let location = warning.location.unwrap();
    assert_eq!(location.document, InputDocument::SecurityContext);
    assert_eq!(location.pointer, "/contract");
    assert_eq!((location.line, location.column), (None, None));
}

#[test]
fn sec_3_9_current_name_reports_no_warning() {
    let context = admit_security_context(&document("[]", "{}"), None).unwrap();
    assert!(context.former_name_warning().is_none());
}

#[test]
fn sec_3_9_a_rejected_context_reports_no_warning() {
    let text = document_named("semantic_compiler.security_context", "u", "[1]", "{}");
    assert!(admit_security_context(&text, None).is_err());
}
