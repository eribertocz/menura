//! The column and field references of a normalized expression tree.

use semantic_compiler_model::expression::{AggregateCall, Expression, Reference};

/// Every column or field reference of `expression`, in the order they are written.
///
/// `AGG(metric)` refers to a metric, not to a column or field, and is not included.
pub(crate) fn references(expression: &Expression) -> Vec<&Reference> {
    let mut found = Vec::new();
    collect(expression, &mut found);
    found
}

fn collect<'tree>(expression: &'tree Expression, found: &mut Vec<&'tree Reference>) {
    for child in children(expression) {
        collect(child, found);
    }
    if let Expression::Reference(reference) = expression {
        found.push(reference);
    }
}

/// The direct subexpressions of `expression`, in the order they are written.
fn children(expression: &Expression) -> Vec<&Expression> {
    match expression {
        Expression::Literal(_)
        | Expression::Reference(_)
        | Expression::MetricAggregate(_)
        | Expression::UserAttribute(_)
        | Expression::CurrentDate
        | Expression::CurrentTime
        | Expression::CurrentTimestamp => Vec::new(),
        Expression::Unary { operand, .. }
        | Expression::IsNull { operand, .. }
        | Expression::Cast { operand, .. }
        | Expression::Extract { operand, .. }
        | Expression::DateTrunc { operand, .. } => vec![operand],
        Expression::Binary { left, right, .. } | Expression::IsDistinctFrom { left, right, .. } => {
            vec![left, right]
        }
        Expression::InList { operand, list, .. } => {
            std::iter::once(operand.as_ref()).chain(list).collect()
        }
        Expression::Between {
            operand, low, high, ..
        } => vec![operand, low, high],
        Expression::Like {
            operand, pattern, ..
        } => vec![operand, pattern],
        Expression::Case {
            operand,
            branches,
            otherwise,
        } => operand
            .as_deref()
            .into_iter()
            .chain(
                branches
                    .iter()
                    .flat_map(|branch| [&branch.condition, &branch.result]),
            )
            .chain(otherwise.as_deref())
            .collect(),
        Expression::DateAdd {
            amount, operand, ..
        } => vec![amount, operand],
        Expression::DateDiff { start, end, .. } => vec![start, end],
        Expression::Function { arguments, .. } => arguments.iter().collect(),
        Expression::Aggregate { call, filter } => aggregate_children(call)
            .into_iter()
            .chain(filter.as_deref())
            .collect(),
    }
}

fn aggregate_children(call: &AggregateCall) -> Vec<&Expression> {
    match call {
        AggregateCall::CountRows => Vec::new(),
        AggregateCall::Simple { argument, .. } | AggregateCall::Distinct { argument, .. } => {
            vec![argument]
        }
        AggregateCall::Percentile {
            fraction, ordering, ..
        } => vec![fraction, ordering],
    }
}

#[cfg(test)]
mod tests {
    use semantic_compiler_expressions::parse;

    use super::references;

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
