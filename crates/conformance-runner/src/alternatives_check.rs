//! The `alternatives` expectation (CS-2.17): the `alternatives` of the first diagnostic of a
//! compile error, compared item by item.

use menura_core::Diagnostic;

/// Why the `alternatives` of `first`, the first diagnostic of the compile error, differ from
/// `expected`, or `None`. An empty `expected` list asserts that the diagnostic has no
/// `alternatives` member (CS-2.17, PLN-5.4).
pub(crate) fn alternatives_mismatch(
    expected: &[String],
    first: Option<&Diagnostic>,
) -> Option<String> {
    let reported = first.and_then(|diagnostic| diagnostic.alternatives.as_deref());
    match reported {
        None if expected.is_empty() => None,
        Some(alternatives) if expected.is_empty() => Some(format!(
            "the first diagnostic has `alternatives` {:?}, expected none",
            alternatives.0
        )),
        None => Some(format!(
            "the first diagnostic has no `alternatives`, expected {expected:?}"
        )),
        Some(alternatives) if alternatives.0 == expected => None,
        Some(alternatives) => Some(format!(
            "the first diagnostic has `alternatives` {:?}, expected {expected:?}",
            alternatives.0
        )),
    }
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::compiled_plan::Alternatives;

    use super::*;

    fn diagnostic(alternatives: Option<&[&str]>) -> Diagnostic {
        Diagnostic {
            code: "E_NAME_NOT_FOUND".to_owned(),
            message: "m".to_owned(),
            location: None,
            alternatives: alternatives.map(|names| {
                Box::new(Alternatives(
                    names.iter().map(|name| (*name).to_owned()).collect(),
                ))
            }),
        }
    }

    fn expected(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn cs_2_17_equal_lists_pass() {
        let first = diagnostic(Some(&["metric:BILLED", "metric:BILLS"]));
        let expected = expected(&["metric:BILLED", "metric:BILLS"]);
        assert_eq!(alternatives_mismatch(&expected, Some(&first)), None);
    }

    #[test]
    fn cs_2_17_another_order_fails() {
        let first = diagnostic(Some(&["metric:BILLS", "metric:BILLED"]));
        let expected = expected(&["metric:BILLED", "metric:BILLS"]);
        assert!(alternatives_mismatch(&expected, Some(&first)).is_some());
    }

    #[test]
    fn cs_2_17_a_missing_list_fails_a_non_empty_expectation() {
        let reason =
            alternatives_mismatch(&expected(&["metric:BILLED"]), Some(&diagnostic(None))).unwrap();
        assert!(reason.contains("no `alternatives`"), "{reason}");
    }

    #[test]
    fn cs_2_17_an_empty_list_asserts_that_the_member_is_absent() {
        assert_eq!(alternatives_mismatch(&[], Some(&diagnostic(None))), None);
        assert_eq!(alternatives_mismatch(&[], None), None);
        let present = diagnostic(Some(&["metric:BILLED"]));
        assert!(alternatives_mismatch(&[], Some(&present)).is_some());
    }
}
