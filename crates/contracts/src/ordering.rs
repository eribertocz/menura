//! CON-5 ordering: normalized names compared as sequences of Unicode code points.

use std::cmp::Ordering;

/// Compares two normalized names by Unicode code point (CON-5).
///
/// This is not UTF-16 order: `"\u{FF61}"` sorts before `"\u{1F600}"` here, after it in UTF-16.
/// RFC 8785 member order, which is UTF-16 order, is a different rule (CON-2).
///
/// ```
/// use std::cmp::Ordering;
/// use menura_contracts::compare_names;
///
/// assert_eq!(compare_names("B", "a"), Ordering::Less);
/// ```
#[must_use]
pub fn compare_names(left: &str, right: &str) -> Ordering {
    left.chars().cmp(right.chars())
}
