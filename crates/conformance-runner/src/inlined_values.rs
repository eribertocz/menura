//! The parameter check of the Security gate (roadmap 7.1a): no compiled plan of the suite carries
//! a value of its query or of its security context outside its `parameters` (E6-4.2, E6-4.11).
//!
//! The values are collected from the case's documents: the query's `time_zone` and
//! `reference_time`, the literals of its expressions (`filter`, the `filters` of its extension,
//! its SQL form), the bounds of its `date_range`, and the scalar values of the security
//! context's `attributes`. The plan's `sql` is then searched for each value that no parameter
//! holds, as a string literal and as a number token.

use menura_contracts::types::compiled_plan::CompiledPlanDocument;
use serde_json::Value as Json;

use crate::intended_literals::{is_generated_on_purpose, is_generator_constant};
use crate::sql_literals::{Literal, LiteralKind, literals, normalized_number};

/// The members of the query envelope whose values are bound parameters: the query time zone and
/// the reference time (E6-4.2c).
const ENVELOPE_VALUES: [&str; 2] = ["time_zone", "reference_time"];

/// A value of the query or of the security context that must be a bound parameter (E6-4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundValue {
    /// Whether it is searched as a string literal or as a number token.
    kind: LiteralKind,
    /// The value, as [`Literal::value`] holds it.
    value: String,
    /// Where it comes from, such as `/query/filter` or `/attributes/region`.
    source: String,
}

/// Why `plan` carries a value of `query` or of `security_context` in its `sql`, or `None`. The
/// reason names the value, where it comes from and where the SQL holds it (line and column,
/// from 1).
///
/// Every value is searched, even one that a parameter also holds, so that a value bound in one
/// place and written in another is caught. The literals of the SQL that are written on purpose
/// are not searched: those of [`is_generated_on_purpose`] and the generator constants of
/// [`is_generator_constant`].
pub(crate) fn inlined_value_mismatch(
    query: Option<&Json>,
    security_context: &[u8],
    plan: &CompiledPlanDocument,
) -> Option<String> {
    let mut values = query.map(query_values).unwrap_or_default();
    values.extend(attribute_values(security_context));
    let sql_literals: Vec<Literal> = literals(&plan.sql)
        .into_iter()
        .filter(|literal| {
            !is_generated_on_purpose(literal) && !is_generator_constant(literal, &plan.sql)
        })
        .collect();
    values.iter().find_map(|value| {
        sql_literals
            .iter()
            .find(|literal| writes(literal, value))
            .map(|literal| describe_leak(value, literal, &plan.sql))
    })
}

/// Whether the SQL literal `literal` writes `value`: a literal of the same kind and value, or,
/// for a number, also a string literal of that number, such as `'10.50'` for `10.5`.
fn writes(literal: &Literal, value: &BoundValue) -> bool {
    match (literal.kind, value.kind) {
        (LiteralKind::Text, LiteralKind::Number) => {
            is_number(&literal.value) && normalized_number(&literal.value) == value.value
        }
        _ => literal.kind == value.kind && literal.value == value.value,
    }
}

/// Whether `text` is a number written with digits, an optional sign and an optional fraction.
fn is_number(text: &str) -> bool {
    let digits = text.trim_start_matches(['+', '-']);
    let (integer, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    !(integer.is_empty() && fraction.is_empty())
        && integer.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}

fn describe_leak(value: &BoundValue, literal: &Literal, sql: &str) -> String {
    let before = &sql[..literal.offset];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |text| text.chars().count())
        + 1;
    format!(
        "the value `{}` of `{}` is written in the SQL at line {line}, column {column}, instead \
         of a bound parameter (E6-4.2)",
        value.value, value.source
    )
}

/// The values of the query document `query` (QRY-2, QRY-3, QRY-4).
fn query_values(query: &Json) -> Vec<BoundValue> {
    let mut values: Vec<BoundValue> = ENVELOPE_VALUES
        .iter()
        .filter_map(|member| {
            let text = query.get(member).and_then(Json::as_str)?;
            Some(BoundValue {
                kind: LiteralKind::Text,
                value: text.to_owned(),
                source: format!("/{member}"),
            })
        })
        .collect();
    if let Some(sql) = query.get("sql").and_then(Json::as_str) {
        values.extend(expression_values(sql, "/sql"));
    }
    let Some(wide) = query.get("query") else {
        return values;
    };
    if let Some(filter) = wide.get("filter").and_then(Json::as_str) {
        values.extend(expression_values(filter, "/query/filter"));
    }
    let extensions = wide
        .get("nonportable_extensions")
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .enumerate();
    for (index, extension) in extensions {
        let pointer = format!("/query/nonportable_extensions/{index}/data");
        values.extend(extension_values(extension.get("data"), &pointer));
    }
    values
}

/// The values of the `data` of an extension entry: its `filters` and its `date_range` bounds.
fn extension_values(data: Option<&Json>, pointer: &str) -> Vec<BoundValue> {
    let mut values = Vec::new();
    let Some(data) = data else {
        return values;
    };
    let filters = data.get("filters").and_then(Json::as_array).into_iter();
    for (index, filter) in filters.flatten().enumerate() {
        if let Some(filter) = filter.as_str() {
            values.extend(expression_values(
                filter,
                &format!("{pointer}/filters/{index}"),
            ));
        }
    }
    for bound in ["from", "to"] {
        let value = data.get("date_range").and_then(|range| range.get(bound));
        if let Some(date) = value.and_then(Json::as_str) {
            values.push(BoundValue {
                kind: LiteralKind::Text,
                value: date.to_owned(),
                source: format!("{pointer}/date_range/{bound}"),
            });
        }
    }
    values
}

/// The literals of a query expression that must be bound (see [`is_generated_on_purpose`]).
fn expression_values(expression: &str, pointer: &str) -> Vec<BoundValue> {
    literals(expression)
        .into_iter()
        .filter(|literal| !is_generated_on_purpose(literal))
        .map(|literal| BoundValue {
            kind: literal.kind,
            value: literal.value,
            source: pointer.to_owned(),
        })
        .collect()
}

/// The scalar values of the `attributes` of the security context, list items included
/// (SEC-3). A context that is not JSON has no values.
fn attribute_values(security_context: &[u8]) -> Vec<BoundValue> {
    let Ok(context) = serde_json::from_slice::<Json>(security_context) else {
        return Vec::new();
    };
    let Some(attributes) = context.get("attributes").and_then(Json::as_object) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for (name, value) in attributes {
        let source = format!("/attributes/{name}");
        let items = match value {
            Json::Array(items) => items.iter().collect(),
            scalar => vec![scalar],
        };
        values.extend(
            items
                .into_iter()
                .flat_map(|item| scalar_values(item, &source)),
        );
    }
    values
}

/// The values to search for of a string or a number of the security context. A string that is
/// a number, such as a `decimal` or a large integer that CON-4 writes as a string (`"0.50"`), is
/// also searched as a number token. Booleans and `null` are not searched.
fn scalar_values(value: &Json, source: &str) -> Vec<BoundValue> {
    let bound = |kind, value| BoundValue {
        kind,
        value,
        source: source.to_owned(),
    };
    match value {
        Json::String(text) if is_number(text) => vec![
            bound(LiteralKind::Text, text.clone()),
            bound(LiteralKind::Number, normalized_number(text)),
        ],
        Json::String(text) => vec![bound(LiteralKind::Text, text.clone())],
        Json::Number(number) => vec![bound(
            LiteralKind::Number,
            normalized_number(&number.to_string()),
        )],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::{LogicalType, Parameter};

    use super::*;

    const PLAN: &str = include_str!("../../../spec/contracts/examples/compiled-plan.example.json");

    fn plan_with_sql(sql: &str) -> CompiledPlanDocument {
        let mut plan: CompiledPlanDocument = serde_json::from_str(PLAN).unwrap();
        plan.sql = sql.to_owned();
        plan.parameters.clear();
        plan
    }

    fn filtered_query(filter: &str) -> Json {
        serde_json::json!({"query": {"metrics": ["m"], "filter": filter}})
    }

    #[test]
    fn e6_4_2_a_query_value_written_in_the_sql_is_caught() {
        let plan = plan_with_sql("SELECT a\nFROM t WHERE region = 'NORTH'");
        let query = filtered_query("sales.region = 'NORTH'");
        let reason = inlined_value_mismatch(Some(&query), b"{}", &plan).unwrap();
        assert!(reason.contains("`NORTH`"), "{reason}");
        assert!(reason.contains("/query/filter"), "{reason}");
        assert!(reason.contains("line 2, column 23"), "{reason}");
    }

    #[test]
    fn e6_4_2_a_bound_query_value_passes() {
        let plan = plan_with_sql("SELECT a FROM t WHERE amount >= CAST($1 AS DECIMAL(4,2))");
        let query = filtered_query("sales.amount >= 10.50");
        assert_eq!(inlined_value_mismatch(Some(&query), b"{}", &plan), None);
    }

    #[test]
    fn e6_4_2_an_inlined_number_of_a_filter_is_caught() {
        let plan = plan_with_sql("SELECT a FROM t WHERE amount >= 10.5");
        let query = serde_json::json!({"query": {"nonportable_extensions": [
            {"vendor_name": "COMMON", "data": {"filters": ["sales.amount >= 10.50"]}}]}});
        let reason = inlined_value_mismatch(Some(&query), b"{}", &plan).unwrap();
        assert!(reason.contains("/filters/0"), "{reason}");
    }

    #[test]
    fn e6_4_2_an_inlined_attribute_is_caught() {
        let plan = plan_with_sql("SELECT a FROM t WHERE region IN ('EU', 'US')");
        let context = br#"{"attributes": {"regions": ["EU"], "tier": 7, "admin": true}}"#;
        let reason = inlined_value_mismatch(None, context, &plan).unwrap();
        assert!(reason.contains("/attributes/regions"), "{reason}");
    }

    #[test]
    fn e6_4_2_an_inlined_date_range_bound_is_caught() {
        let plan = plan_with_sql("SELECT a FROM t WHERE d >= DATE '2026-03-01'");
        let query = serde_json::json!({"query": {"nonportable_extensions": [{"data":
            {"date_range": {"dimension": "t.d", "from": "2026-03-01", "to": "2026-03-31"}}}]}});
        let reason = inlined_value_mismatch(Some(&query), b"{}", &plan).unwrap();
        assert!(reason.contains("date_range/from"), "{reason}");
    }

    #[test]
    fn e6_4_2_a_number_written_as_a_string_literal_is_caught() {
        let plan = plan_with_sql("SELECT a FROM t WHERE amount >= CAST('10.50' AS DECIMAL(4,2))");
        let query = filtered_query("sales.amount >= 10.50");
        assert!(inlined_value_mismatch(Some(&query), b"{}", &plan).is_some());
    }

    #[test]
    fn e6_4_2c_an_inlined_time_zone_is_caught() {
        let plan = plan_with_sql("SELECT a AT TIME ZONE 'Asia/Tokyo' FROM t");
        let query = serde_json::json!({"time_zone": "Asia/Tokyo", "query": {"metrics": ["m"]}});
        let reason = inlined_value_mismatch(Some(&query), b"{}", &plan).unwrap();
        assert!(reason.contains("/time_zone"), "{reason}");
    }

    #[test]
    fn e6_4_2_a_value_both_bound_and_written_is_caught() {
        let mut plan = plan_with_sql(
            "SELECT a FROM t WHERE region = CAST($1 AS VARCHAR) UNION ALL SELECT a FROM u WHERE \
             region = 'NORTH'",
        );
        plan.parameters = vec![Parameter {
            position: 1,
            logical_type: LogicalType::Text,
            value: serde_json::json!("NORTH"),
        }];
        let query = filtered_query("sales.region = 'NORTH'");
        let reason = inlined_value_mismatch(Some(&query), b"{}", &plan).unwrap();
        assert!(reason.contains("`NORTH`"), "{reason}");
    }

    #[test]
    fn e6_4_2_a_decimal_attribute_written_as_a_number_is_caught() {
        let context = br#"{"attributes": {"rate": "0.50"}}"#;
        for sql in [
            "SELECT a FROM t WHERE r = 0.5",
            "SELECT a FROM t WHERE r = CAST(0.50 AS DECIMAL(2,2))",
        ] {
            let reason = inlined_value_mismatch(None, context, &plan_with_sql(sql)).unwrap();
            assert!(reason.contains("/attributes/rate"), "{reason}");
        }
    }

    #[test]
    fn e6_4_11_the_constants_of_the_generator_are_not_values() {
        let plan = plan_with_sql(
            "SELECT CASE WHEN CAST(d AS DOUBLE) = 0 THEN NULL ELSE n / d END, \
             p + INTERVAL 1 MONTH - INTERVAL 1 DAY, CAST(s AS DECIMAL(38,2)) FROM t GROUP BY 1 \
             QUALIFY RANK() OVER (PARTITION BY k ORDER BY (p) DESC) = 1",
        );
        let query = filtered_query("sales.quantity >= 1 AND sales.amount > 0 AND sales.cost > 2");
        assert_eq!(inlined_value_mismatch(Some(&query), b"{}", &plan), None);
    }

    #[test]
    fn e6_4_2_a_one_compared_outside_a_window_is_caught() {
        let plan = plan_with_sql("SELECT a FROM t WHERE (q) = 1");
        let query = filtered_query("sales.quantity = 1");
        assert!(inlined_value_mismatch(Some(&query), b"{}", &plan).is_some());
    }

    #[test]
    fn e6_4_2_a_zero_outside_the_division_guard_is_caught() {
        let plan = plan_with_sql("SELECT CASE WHEN x = 0 THEN 'none' END FROM t");
        let query = filtered_query("sales.amount > 0");
        assert!(inlined_value_mismatch(Some(&query), b"{}", &plan).is_some());
    }

    #[test]
    fn e6_4_11_a_limit_integer_of_the_sql_form_is_a_literal() {
        let plan = plan_with_sql("SELECT a FROM t ORDER BY a LIMIT 10");
        let query = serde_json::json!({"sql": "SELECT a FROM m.t ORDER BY a LIMIT 10"});
        assert_eq!(inlined_value_mismatch(Some(&query), b"{}", &plan), None);
    }
}
