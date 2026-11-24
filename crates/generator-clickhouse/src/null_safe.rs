//! Null-safe equality, in place of `IS NOT DISTINCT FROM`, which `ClickHouse` does not have
//! (E6-7.4): two NULL values are equal and a NULL value never equals a non-NULL one.

/// The null-safe equality of `left` and `right` in a `JOIN … ON` condition:
/// `isNotDistinctFrom(left, right)`, which `ClickHouse` reads as an equality key of the join.
pub(crate) fn join_equal(left: &str, right: &str) -> String {
    format!("isNotDistinctFrom({left}, {right})")
}

/// The null-safe equality of `left` and `right` anywhere else, never NULL: `left = right` when
/// both are known, true when both are NULL, false when only one is.
pub(crate) fn not_distinct(left: &str, right: &str) -> String {
    format!("coalesce({left} = {right}, {left} IS NULL AND {right} IS NULL)")
}

#[cfg(test)]
mod tests {
    use super::{join_equal, not_distinct};

    #[test]
    fn e6_7_4_keys_are_compared_null_safe() {
        assert_eq!(
            join_equal("\"a\"", "\"b\""),
            "isNotDistinctFrom(\"a\", \"b\")"
        );
        assert_eq!(
            not_distinct("\"a\"", "\"b\""),
            "coalesce(\"a\" = \"b\", \"a\" IS NULL AND \"b\" IS NULL)"
        );
    }
}
