//! Unit tests of the SQL form (E1-4), named after their rule IDs.

use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::Identifier;
use menura_representations::columns::OutputColumn;
use menura_representations::query_tree::{
    QueryDimensionKind, QueryMetricKind, QueryReference, QueryTree, ReferencePart,
};
use menura_representations::time::{QueryTime, TimeZoneName};

use super::clauses::statement;
use super::lexer::tokenize;
use super::relation::relation;
use super::sql_tree;
use crate::error::{ParseError, ParseErrorCode};

fn time() -> QueryTime {
    QueryTime {
        time_zone: TimeZoneName::new("UTC"),
        reference_time: None,
        reference_date: None,
    }
}

fn tree(sql: &str) -> QueryTree {
    match sql_tree(sql, time()) {
        Ok(tree) => tree,
        Err(error) => panic!("`{sql}` should parse: {error}"),
    }
}

fn error(sql: &str) -> ParseError {
    match sql_tree(sql, time()) {
        Ok(_) => panic!("`{sql}` should be rejected"),
        Err(error) => error,
    }
}

fn assert_eqry_05(sql: &str) {
    let error = error(sql);
    assert_eq!(error.code, ParseErrorCode::Eqry05, "`{sql}`: {error}");
    assert_eq!(error.pointer, "/sql", "`{sql}`");
}

fn output_names(tree: &QueryTree) -> Vec<String> {
    tree.columns()
        .iter()
        .map(|column| match column {
            OutputColumn::Dimension(dimension) => dimension.output_name.as_str().to_owned(),
            OutputColumn::Metric(metric) => metric.output_name.as_str().to_owned(),
        })
        .collect()
}

fn regular(text: &str) -> ReferencePart {
    ReferencePart::Regular(text.to_owned())
}

#[test]
fn e1_4_parses_the_subset_into_the_query_tree() {
    let tree = tree(
        "SELECT s.region, DATE_TRUNC('month', s.sale_date) AS month, AGG(s.total_amount) AS total \
         FROM slice.sales AS s WHERE s.voided = FALSE GROUP BY 1, 2 ORDER BY total DESC LIMIT 5",
    );
    assert_eq!(output_names(&tree), ["region", "month", "total"]);
    let columns: Vec<_> = tree.columns().iter().collect();
    assert!(matches!(
        columns[0],
        OutputColumn::Dimension(dimension) if matches!(&dimension.kind, QueryDimensionKind::Field { reference }
            if reference.parts() == [regular("s"), regular("region")])
    ));
    assert!(matches!(
        columns[1],
        OutputColumn::Dimension(dimension) if matches!(dimension.kind, QueryDimensionKind::Expression(_))
    ));
    assert!(matches!(
        columns[2],
        OutputColumn::Metric(metric) if metric.kind == QueryMetricKind::Reference(
            QueryReference::new(vec![regular("s"), regular("total_amount")]).unwrap_or_else(|| unreachable!())
        )
    ));
    assert_eq!(tree.filters().len(), 1);
    assert_eq!(tree.order().len(), 1);
    assert_eq!(tree.order()[0].output.as_str(), "total");
    assert_eq!(tree.order()[0].direction, Direction::Desc);
    assert_eq!(tree.limit().map(|limit| limit.rows), Some(5));
}

#[test]
fn e1_4_accepts_comments_a_trailing_semicolon_and_measure() {
    let tree = tree(
        "-- revenue by region\nSELECT region, /* the metric */ MEASURE(total_amount) AS total \
         FROM slice.sales GROUP BY ALL;",
    );
    assert_eq!(output_names(&tree), ["region", "total"]);
}

#[test]
fn e1_4_an_expression_over_agg_terms_is_an_inline_metric() {
    let tree = tree("SELECT AGG(total_amount) / AGG(sale_count) AS average FROM slice.sales");
    assert!(matches!(
        tree.columns().first(),
        Some(OutputColumn::Metric(metric)) if matches!(metric.kind, QueryMetricKind::Inline(_))
    ));
}

#[test]
fn e1_4_rejects_constructs_outside_the_subset() {
    for sql in [
        "WITH t AS (SELECT 1) SELECT AGG(m) AS m FROM slice.sales",
        "SELECT DISTINCT region, AGG(m) AS m FROM slice.sales GROUP BY region",
        "SELECT * FROM slice.sales",
        "SELECT AGG(m) AS m FROM slice.sales UNION SELECT AGG(m) AS m FROM slice.sales",
        "SELECT AGG(m) AS m FROM slice.sales JOIN slice.targets ON TRUE",
        "SELECT AGG(m) AS m FROM slice.sales, slice.targets",
        "SELECT AGG(m) AS m FROM (SELECT 1) AS t",
        "SELECT AGG(m) AS m FROM sales",
        "SELECT AGG(m) AS m FROM slice.sales HAVING AGG(m) > 1",
        "SELECT AGG(m) AS m FROM slice.sales WHERE region IN (SELECT region FROM slice.targets)",
        "SELECT SUM(amount) AS m FROM slice.sales",
        "SELECT AGG(m) AS m FROM slice.sales WHERE AGG(m) > 1",
        "SELECT AGG(m) AS m FROM slice.sales LIMIT 0",
        "SELECT AGG(m) AS m FROM slice.sales LIMIT 1.5",
        "SELECT AGG(m) AS m FROM slice.sales OFFSET 2",
        "SELECT AGG(m) AS m FROM slice.sales; SELECT AGG(m) AS m FROM slice.sales",
    ] {
        assert_eqry_05(sql);
    }
}

#[test]
fn e1_4_3_result_columns_keep_the_select_order() {
    let tree = tree("SELECT AGG(sale_count) AS n, region, channel FROM slice.sales GROUP BY ALL");
    assert_eq!(output_names(&tree), ["n", "region", "channel"]);
}

#[test]
fn e1_4_4_date_trunc_is_a_dimension_expression() {
    let tree = tree(
        "SELECT DATE_TRUNC('week', sold_at) AS week, AGG(sale_count) AS n FROM slice.sales \
         GROUP BY DATE_TRUNC('week', sold_at)",
    );
    assert!(matches!(
        tree.columns().first(),
        Some(OutputColumn::Dimension(dimension)) if matches!(dimension.kind, QueryDimensionKind::Expression(_))
    ));
}

#[test]
fn e1_4_5_unparsable_text_reports_line_and_column_in_code_points() {
    let error = error("SELECT AGG(m) AS m\nFROM slice.sales\nWHERE région = 'é' AND");
    assert_eq!(error.code, ParseErrorCode::Eqry05);
    assert_eq!(error.pointer, "/sql");
    assert!(
        error.message.contains("line 3, column 23"),
        "{}",
        error.message
    );
}

#[test]
fn e1_4_5_unterminated_text_does_not_parse() {
    let error = error("SELECT AGG(m) AS m FROM slice.sales WHERE region = 'NORTH");
    assert_eq!(error.code, ParseErrorCode::Eqry05);
    assert!(
        error.message.contains("line 1, column 52"),
        "{}",
        error.message
    );
}

#[test]
fn e1_4_5_clauses_out_of_order_do_not_parse() {
    assert_eqry_05("SELECT AGG(m) AS m WHERE region = 'NORTH' FROM slice.sales");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales ORDER BY 1 GROUP BY 1");
    assert_eqry_05("");
    assert_eqry_05("SELECT FROM slice.sales");
}

#[test]
fn e1_4_6_a_query_without_agg_is_rejected() {
    assert_eqry_05("SELECT region FROM slice.sales GROUP BY region");
}

#[test]
fn e1_4_7_every_non_aggregated_item_is_grouped() {
    assert_eqry_05("SELECT region, channel, AGG(m) AS m FROM slice.sales GROUP BY region");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 3");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 0");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 2");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY region, channel");
}

#[test]
fn e1_4_7_group_by_accepts_expressions_positions_names_and_all() {
    for sql in [
        "SELECT region, AGG(m) AS m FROM slice.sales GROUP BY REGION",
        "SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 1",
        "SELECT UPPER(region) AS r, AGG(m) AS m FROM slice.sales GROUP BY r",
        "SELECT UPPER(region) AS r, AGG(m) AS m FROM slice.sales GROUP BY upper( region )",
        "SELECT region, channel, AGG(m) AS m FROM slice.sales GROUP BY ALL",
        "SELECT AGG(m) AS m FROM slice.sales",
    ] {
        tree(sql);
    }
}

#[test]
fn e1_4_8_an_item_mixing_fields_and_agg_is_rejected() {
    assert_eqry_05("SELECT region, AGG(m) + quantity AS mixed FROM slice.sales GROUP BY region");
}

#[test]
fn e1_4_10_order_by_defaults_to_ascending_nulls_last() {
    let tree = tree(
        "SELECT region, channel, AGG(m) AS m FROM slice.sales GROUP BY ALL \
         ORDER BY region, 2 DESC, m NULLS FIRST",
    );
    let order: Vec<_> = tree
        .order()
        .iter()
        .map(|item| (item.output.as_str(), item.direction, item.nulls))
        .collect();
    assert_eq!(
        order,
        [
            ("region", Direction::Asc, NullsOrder::Last),
            ("channel", Direction::Desc, NullsOrder::Last),
            ("m", Direction::Asc, NullsOrder::First),
        ]
    );
}

#[test]
fn e1_4_order_by_an_expression_sorts_by_its_output() {
    let tree = tree(
        "SELECT UPPER(region) AS r, AGG(m) AS m FROM slice.sales GROUP BY 1 ORDER BY UPPER(region)",
    );
    assert_eq!(tree.order()[0].output.as_str(), "r");
}

#[test]
fn e1_4_order_by_an_unknown_item_is_rejected() {
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 1 ORDER BY channel");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 1 ORDER BY 3");
    assert_eqry_05("SELECT region, AGG(m) AS m FROM slice.sales GROUP BY 1 ORDER BY AGG(m)");
}

#[test]
fn eqry_13_an_item_that_is_not_a_plain_field_needs_an_alias() {
    for sql in [
        "SELECT AGG(m) FROM slice.sales",
        "SELECT UPPER(region), AGG(m) AS m FROM slice.sales GROUP BY 1",
    ] {
        let error = error(sql);
        assert_eq!(error.code, ParseErrorCode::Eqry13, "`{sql}`: {error}");
        assert_eq!(error.pointer, "/sql");
    }
}

#[test]
fn e1_5_1_a_plain_field_takes_the_last_part_of_its_reference() {
    let tree = tree("SELECT s.\"Net Region\", AGG(m) AS \"Total\" FROM slice.sales s GROUP BY 1");
    assert_eq!(output_names(&tree), ["Net Region", "Total"]);
}

#[test]
fn e1_5_origins_carry_the_position_in_the_sql_text() {
    let tree = tree("SELECT region,\n  AGG(m) AS m FROM slice.sales GROUP BY 1");
    let positions: Vec<_> = tree
        .columns()
        .iter()
        .map(|column| match column {
            OutputColumn::Dimension(dimension) => dimension.origin.position(),
            OutputColumn::Metric(metric) => metric.origin.position(),
        })
        .map(|position| position.map(|position| (position.line(), position.column())))
        .collect();
    assert_eq!(positions, [Some((1, 8)), Some((2, 3))]);
}

#[test]
fn e1_4_reads_the_from_relation_with_or_without_as() {
    for (sql, alias) in [
        ("SELECT AGG(m) AS m FROM slice.\"Sales\" AS s", Some("s")),
        ("SELECT AGG(m) AS m FROM slice.\"Sales\" s", Some("s")),
        ("SELECT AGG(m) AS m FROM slice.\"Sales\"", None),
    ] {
        let tokens = tokenize(sql).unwrap_or_else(|_| unreachable!());
        let statement = statement(sql, &tokens).unwrap_or_else(|_| unreachable!());
        let relation = relation(sql, &tokens, &statement.from).unwrap_or_else(|_| unreachable!());
        assert_eq!(relation.model.text(), "slice");
        assert_eq!(relation.dataset.text(), "Sales");
        assert!(relation.dataset.is_delimited());
        assert_eq!(relation.alias.as_ref().map(Identifier::text), alias);
    }
}

#[test]
fn e1_4_is_distinct_from_in_where_starts_no_clause() {
    let tree = tree("SELECT AGG(m) AS m FROM slice.sales WHERE channel IS NOT DISTINCT FROM 'web'");
    assert_eq!(tree.filters().len(), 1);
}
