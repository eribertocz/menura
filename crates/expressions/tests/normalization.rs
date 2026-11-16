//! E0H-1.3: formulas that differ only in whitespace, keyword case or redundant parentheses
//! produce the same tree.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use support::tree;

#[test]
fn e0h_1_3_format_never_changes_the_tree() {
    let groups: &[&[&str]] = &[
        &[
            "SUM(orders.amount) FILTER (WHERE orders.status = 'OPEN')",
            "sum ( ( orders.amount ) )\n\tfilter(where((orders.status)='OPEN'))",
            "Sum(orders.amount)FILTER(WHERE orders.status='OPEN') -- trailing comment",
        ],
        &[
            "CASE WHEN a IS NOT NULL THEN COALESCE(a, 0) ELSE NULL END",
            "case when (a is not null) then coalesce((a), (0)) else null end",
            "CASE /* comment */ WHEN a IS NOT NULL THEN COALESCE ( a , 0 ) ELSE NULL END",
        ],
        &["a + b * c", "a + (b * c)", "((a) + ((b) * (c)))"],
        &["CASE a WHEN 1 THEN 2 END", "case (a) when (1) then (2) end"],
        &["CAST(a AS NUMERIC(10,2))", "cast(a as decimal ( 10 , 2 ))"],
        &[
            "PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a)",
            "percentile_cont(.5) within group (order by a asc)",
        ],
        &["DATE '2026-01-05'", "date '2026-01-05'"],
        &["007", "7"],
    ];
    for group in groups {
        let first = tree(group[0]);
        for text in &group[1..] {
            assert_eq!(tree(text), first, "{text:?} differs from {:?}", group[0]);
        }
    }
}

#[test]
fn e0h_1_3_meaningful_differences_change_the_tree() {
    let pairs = [
        ("a - b - c", "a - (b - c)"),
        ("(a + b) * c", "a + b * c"),
        ("0.5", "0.50"),
        ("'Open'", "'OPEN'"),
        ("amount", "\"amount\""),
        ("SUM(a)", "SUM(DISTINCT a)"),
    ];
    for (left, right) in pairs {
        assert_ne!(tree(left), tree(right), "{left:?} and {right:?}");
    }
}
