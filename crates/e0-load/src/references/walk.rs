//! The column and field references, and the `AGG(…)` terms, of a normalized expression tree.

use menura_model::expression::{Expression, Reference};

use crate::expression_tree::nodes;

/// Every column or field reference of `expression`, in the order they are written.
///
/// `AGG(metric)` refers to a metric, not to a column or field, and is not included.
pub(crate) fn references(expression: &Expression) -> Vec<&Reference> {
    nodes(expression)
        .into_iter()
        .filter_map(|node| match node {
            Expression::Reference(reference) => Some(reference),
            _ => None,
        })
        .collect()
}

/// The metric every `AGG(…)` term of `expression` names, in the order they are written
/// (DER-2.1).
pub(crate) fn metric_aggregates(expression: &Expression) -> Vec<&Reference> {
    nodes(expression)
        .into_iter()
        .filter_map(|node| match node {
            Expression::MetricAggregate(metric) => Some(metric),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use menura_expressions::parse;

    use super::{metric_aggregates, references};

    fn names(text: &str) -> Vec<String> {
        let tree = parse(text).expect("the expression parses");
        references(&tree)
            .iter()
            .map(|reference| {
                let name = reference.name.text();
                match &reference.qualifier {
                    Some(qualifier) => format!("{}.{name}", qualifier.text()),
                    None => name.to_owned(),
                }
            })
            .collect()
    }

    #[test]
    fn der_2_1_every_metric_aggregate_is_found_in_written_order() {
        let tree = parse("CASE WHEN AGG(a) > 0 THEN AGG(o.b) / NULLIF(AGG(c), 0) END + AGG(a)")
            .expect("the expression parses");
        let found: Vec<String> = metric_aggregates(&tree)
            .iter()
            .map(|reference| reference.name.text().to_owned())
            .collect();
        assert_eq!(found, ["a", "b", "c", "a"]);
        let tree = parse("SUM(o.a) + o.b").expect("the expression parses");
        assert!(metric_aggregates(&tree).is_empty());
    }

    #[test]
    fn every_reference_is_found_in_written_order() {
        assert_eq!(
            names(
                "CASE WHEN a IN (b, c) AND d BETWEEN e AND f THEN CAST(g AS INTEGER) ELSE \
                 COALESCE(h, i) END || j"
            ),
            ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]
        );
        assert_eq!(
            names("SUM(o.a) FILTER (WHERE o.b LIKE 'x%') + COUNT(*) + AGG(m)"),
            ["o.a", "o.b"]
        );
        assert_eq!(
            names("PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY o.a) - DATEDIFF(day, o.b, o.c)"),
            ["o.a", "o.b", "o.c"]
        );
        assert_eq!(
            names(
                "CASE a WHEN 1 THEN DATEADD(day, b, c) END IS NOT DISTINCT FROM EXTRACT(YEAR FROM d)"
            ),
            ["a", "b", "c", "d"]
        );
    }
}
