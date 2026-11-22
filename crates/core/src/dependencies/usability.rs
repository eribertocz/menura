//! The usability of an element (DEP-3.3) and the query error codes its hash covers (DEP-3.10).

use std::collections::BTreeSet;

use menura_model::internal_model::DiagnosticCode;

/// The query error code of a dataset that E0.h propagates from an unusable relationship
/// (E0-4.2a).
pub(super) const UNREACHABLE_ROW_ACCESS: &str = "E0H-01";

/// The code a query that reads an element with `query_errors` gets: the first in the order of
/// E2-5 row 12a, or the first code when row 12a reports none; `None` for a usable element.
pub(super) fn query_error_code(query_errors: &[DiagnosticCode]) -> Option<String> {
    query_errors
        .iter()
        .min_by_key(|code| row_12a_rank(code))
        .map(|code| code.as_str().to_owned())
}

/// The codes of `query_errors` that are the element's own: those not propagated from
/// `upstream`, the codes of the elements it reads (E0-4.2a, DEP-3.10).
pub(super) fn own_codes<'code>(
    query_errors: &'code [DiagnosticCode],
    upstream: &BTreeSet<DiagnosticCode>,
) -> Vec<&'code DiagnosticCode> {
    query_errors
        .iter()
        .filter(|code| !upstream.contains(*code))
        .collect()
}

/// The place of `code` in the order of E2-5 row 12a: `E0B-08`, `E0B-09`, `E0D-03`–`E0D-07`,
/// `E0F-*`, `E0G-01`, `E0G-02`, `E0C-37`, `E0H-01`, then any other code; by code within a
/// group.
fn row_12a_rank(code: &DiagnosticCode) -> (u8, &str) {
    let text = code.as_str();
    let group = match text {
        "E0B-08" => 0,
        "E0B-09" => 1,
        "E0D-03" | "E0D-04" | "E0D-05" | "E0D-06" | "E0D-07" => 2,
        _ if text.starts_with("E0F-") => 3,
        "E0G-01" => 4,
        "E0G-02" => 5,
        "E0C-37" => 6,
        UNREACHABLE_ROW_ACCESS => 7,
        _ => 8,
    };
    (group, text)
}

#[cfg(test)]
mod tests {
    use menura_model::internal_model::DiagnosticCode;

    use super::query_error_code;

    #[test]
    fn dep_3_3_the_query_error_code_follows_e2_row_12a() {
        let codes = [
            DiagnosticCode::new("E0D-04"),
            DiagnosticCode::new("E0F-01"),
            DiagnosticCode::new("E0H-01"),
        ];
        assert_eq!(query_error_code(&codes).as_deref(), Some("E0D-04"));
        assert_eq!(query_error_code(&[]), None);
    }
}
