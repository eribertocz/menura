//! The order of the checks (E1-4.11), the expression codes (E1-4.12) and P5.

use menura_model::expression::Identifier;
use menura_representations::columns::OutputColumn;
use menura_representations::query_tree::QueryTree;

use super::{assert_eqry_05, error, tree};
use crate::error::ParseErrorCode;

#[test]
fn e1_4_11_group_by_never_names_an_output() {
    assert_eqry_05("SELECT UPPER(region) AS r, AGG(m) AS m FROM slice.sales GROUP BY r");
}

#[test]
fn e1_4_11_reports_the_first_failure_in_order() {
    let error = |sql: &str| {
        let error = error(sql);
        (error.code, error.message)
    };
    let (code, message) = error("SELECT SUM(amount) AS a, AGG(m) FROM slice.sales");
    assert_eq!(code, ParseErrorCode::Eqry05, "{message}");
    assert!(message.contains("aggregate function"), "{message}");
    let (code, message) = error("SELECT region, AGG(m) FROM slice.sales GROUP BY 5");
    assert_eq!(code, ParseErrorCode::Eqry13, "{message}");
    let (code, message) = error("SELECT region FROM slice.sales WHERE AGG(m) > 1");
    assert!(message.contains("no AGG"), "{code}: {message}");
    let (code, message) = error("SELECT region, AGG(m) AS m FROM slice.sales WHERE AGG(m) > 1");
    assert!(message.contains("WHERE"), "{code}: {message}");
    let (code, message) =
        error("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 1 ORDER BY channel LIMIT 0");
    assert!(message.contains("ORDER BY"), "{code}: {message}");
    let (code, message) =
        error("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 1 ORDER BY 1 LIMIT 0");
    assert!(message.contains("LIMIT"), "{code}: {message}");
}

#[test]
fn e1_4_12_a_parsed_but_invalid_expression_keeps_its_wide_table_code() {
    for sql in [
        "SELECT RANDOM() AS r, AGG(m) AS m FROM slice.sales GROUP BY 1",
        "SELECT AGG(m) AS m FROM slice.sales WHERE region = RANDOM()",
    ] {
        let error = error(sql);
        assert_eq!(error.code, ParseErrorCode::E0f02, "`{sql}`: {error}");
        assert_eq!(error.pointer, "/sql");
    }
}

#[test]
fn e1_5_the_tree_carries_the_from_relation() {
    let tree = tree("SELECT AGG(m) AS m\nFROM slice.\"Sales\" AS s");
    let relation = tree.relation();
    assert_eq!(
        relation.map(|relation| relation.model.text()),
        Some("slice")
    );
    assert_eq!(
        relation.map(|relation| relation.dataset.text()),
        Some("Sales")
    );
    assert_eq!(
        relation
            .and_then(|relation| relation.alias.as_ref())
            .map(Identifier::text),
        Some("s")
    );
    assert_eq!(
        relation
            .and_then(|relation| relation.origin.position())
            .map(|position| (position.line(), position.column())),
        Some((2, 6))
    );
}

#[test]
fn e1_4_11_a_parse_error_wins_over_a_later_construct_in_text_order() {
    let error = error(
        "SELECT a +* b AS c, AGG(m) AS m FROM slice.sales JOIN slice.targets ON TRUE GROUP BY 1",
    );
    assert_eq!(error.code, ParseErrorCode::Eqry05);
    assert!(
        error.message.contains("does not parse"),
        "{}",
        error.message
    );
    assert!(
        error.message.contains("line 1, column 1"),
        "{}",
        error.message
    );
}

#[test]
fn e1_4_11_a_parse_error_wins_over_a_later_unclosed_literal() {
    let error = error("SELECT a +* b AS c, AGG(m) AS m FROM slice.sales WHERE region = 'NORTH");
    assert!(
        error.message.contains("does not parse"),
        "{}",
        error.message
    );
    assert!(!error.message.contains("literal"), "{}", error.message);
}

#[test]
fn e1_4_11_a_construct_wins_over_a_later_parse_error() {
    let error = error("SELECT AGG(m) AS m FROM slice.sales JOIN slice.targets WHERE a +* b");
    assert!(error.message.contains("JOIN"), "{}", error.message);
}

#[test]
fn e1_4_11_an_e1_4_12_error_is_reported_at_the_step_of_its_clause() {
    let first = error("SELECT region, AGG(m) FROM slice.sales GROUP BY 1 ORDER BY RANDOM()");
    assert_eq!(first.code, ParseErrorCode::Eqry13, "{first}");
    let second = error("SELECT RANDOM() AS r, AGG(m) FROM slice.sales GROUP BY 1");
    assert_eq!(second.code, ParseErrorCode::E0f02, "{second}");
}

#[test]
fn e1_4_11_a_qualified_and_an_unqualified_field_are_the_same_item() {
    let qualified = tree(
        "SELECT region, s.channel, AGG(m) AS m FROM slice.sales AS s \
         GROUP BY s.region, channel ORDER BY sales.region, s.channel",
    );
    let order: Vec<_> = qualified
        .order()
        .iter()
        .map(|item| item.output.as_str())
        .collect();
    assert_eq!(order, ["region", "channel"]);
    tree("SELECT sales.region, AGG(m) AS m FROM slice.sales GROUP BY region");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales AS s GROUP BY t.region");
}

#[test]
fn e1_4_11_limit_is_at_most_two_to_the_63_minus_one() {
    let base = "SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 1 ORDER BY 1 LIMIT ";
    let largest = tree(&format!("{base}9223372036854775807"));
    assert_eq!(
        largest.limit().map(|limit| limit.rows),
        Some(9_223_372_036_854_775_807)
    );
    assert_eqry_05(&format!("{base}9223372036854775808"));
}

#[test]
fn e1_4_5_an_alias_missing_after_as_is_located_at_the_next_clause() {
    let error = error("SELECT AGG(m) AS m FROM slice.sales AS WHERE region = 'NORTH'");
    assert!(
        error.message.contains("line 1, column 40"),
        "{}",
        error.message
    );
}

/// What a tree says about its output, without the origins of its elements.
fn meaning(tree: &QueryTree) -> String {
    let columns: Vec<String> = tree
        .columns()
        .iter()
        .map(|column| match column {
            OutputColumn::Dimension(dimension) => {
                format!("{:?} {}", dimension.kind, dimension.output_name.as_str())
            }
            OutputColumn::Metric(metric) => {
                format!("{:?} {}", metric.kind, metric.output_name.as_str())
            }
        })
        .collect();
    let filters: Vec<_> = tree
        .filters()
        .iter()
        .map(|filter| &filter.expression)
        .collect();
    let order: Vec<_> = tree
        .order()
        .iter()
        .map(|item| (item.output.as_str(), item.direction, item.nulls))
        .collect();
    format!(
        "{columns:?} {filters:?} {order:?} {:?} {:?}",
        tree.limit().map(|limit| limit.rows),
        tree.time()
    )
}

#[test]
fn e1_4_1_p5_the_sql_tree_equals_the_wide_table_tree_of_the_same_query() {
    let wide_table = crate::parse_query(
        r#"{
            "contract": "menura.query", "contract_version": "1.1.0", "time_zone": "UTC",
            "query": {
                "dimensions": ["sales.region"],
                "metrics": [{"metric": "total_amount", "alias": "total"}],
                "filter": "sales.voided = FALSE",
                "nonportable_extensions": [{"vendor_name": "COMMON", "data": {
                    "namespace": "menura", "version": 1,
                    "order_by": [{"output": "total", "direction": "desc", "nulls": "last"}],
                    "limit": 5
                }}]
            }
        }"#,
    )
    .unwrap_or_else(|error| panic!("the wide-table query should parse: {error}"));
    let sql = tree(
        "SELECT sales.region, AGG(total_amount) AS total FROM slice.sales \
         WHERE sales.voided = FALSE GROUP BY 1 ORDER BY total DESC LIMIT 5",
    );
    assert_eq!(meaning(&sql), meaning(&wide_table.tree));
}
