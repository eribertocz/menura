//! Builds the catalog schema document of an Apache Ossie case from the tables its fixture created
//! (CS-4.4), mapping each `DuckDB` type as the CAT-4.4 table states.

use serde_json::{Value, json};

use crate::ossie::created_tables::{CreatedColumn, CreatedRelation};

/// The catalog schema document of the case `case_key`, describing `relations` as created by
/// `DuckDB` `engine_version`.
pub(crate) fn catalog_schema_document(
    case_key: &str,
    engine_version: &str,
    relations: &[CreatedRelation],
) -> Value {
    json!({
        "contract": "menura.catalog_schema",
        "contract_version": "1.1.0",
        "document_id": {
            "origin": format!("conformance:ossie/{case_key}"),
            "snapshot": case_key,
        },
        "engine": {"name": "duckdb", "version": engine_version},
        "relations": relations.iter().map(relation_entry).collect::<Vec<_>>(),
    })
}

fn relation_entry(relation: &CreatedRelation) -> Value {
    json!({
        "source": relation.source,
        "columns": relation.columns.iter().map(column_entry).collect::<Vec<_>>(),
    })
}

fn column_entry(column: &CreatedColumn) -> Value {
    json!({
        "name": column.name,
        "logical_type": logical_type(&column.physical_type),
        "physical_type": column.physical_type,
        "nullable": column.nullable,
    })
}

/// The logical type of the `DuckDB` type `physical_type` (CAT-4.4); every type the table does not
/// list is `unsupported`.
pub(crate) fn logical_type(physical_type: &str) -> Value {
    let name = physical_type.trim();
    if let Some(element) = name.strip_suffix("[]") {
        return match logical_type(element) {
            unsupported if unsupported["kind"] == "unsupported" => unsupported,
            element => json!({"kind": "list", "element": element}),
        };
    }
    if let Some(decimal) = decimal_type(name) {
        return decimal;
    }
    scalar_type(name).unwrap_or_else(|| json!({"kind": "unsupported"}))
}

/// `DECIMAL(p,s)` as `decimal` with precision `p` and scale `s`.
fn decimal_type(name: &str) -> Option<Value> {
    let arguments = name.strip_prefix("DECIMAL(")?.strip_suffix(')')?;
    let (precision, scale) = arguments.split_once(',')?;
    let precision: u8 = precision.trim().parse().ok()?;
    let scale: u8 = scale.trim().parse().ok()?;
    Some(json!({"kind": "decimal", "precision": precision, "scale": scale}))
}

/// The types of the CAT-4.4 table that take no parameter in their name.
fn scalar_type(name: &str) -> Option<Value> {
    let integer =
        |bits: u8, signed: bool| json!({"kind": "integer", "bits": bits, "signed": signed});
    let timestamp = |precision: &str, with_time_zone: bool| json!({"kind": "timestamp", "precision": precision, "with_time_zone": with_time_zone});
    let kind = |kind: &str| json!({ "kind": kind });
    Some(match name {
        "BOOLEAN" => kind("boolean"),
        "TINYINT" => integer(8, true),
        "SMALLINT" => integer(16, true),
        "INTEGER" => integer(32, true),
        "BIGINT" => integer(64, true),
        "HUGEINT" => integer(128, true),
        "UTINYINT" => integer(8, false),
        "USMALLINT" => integer(16, false),
        "UINTEGER" => integer(32, false),
        "UBIGINT" => integer(64, false),
        "UHUGEINT" => integer(128, false),
        "FLOAT" => json!({"kind": "float", "bits": 32}),
        "DOUBLE" => json!({"kind": "float", "bits": 64}),
        "VARCHAR" => kind("text"),
        "BLOB" => kind("binary"),
        "UUID" => kind("uuid"),
        "DATE" => kind("date"),
        "TIME" => kind("time"),
        "INTERVAL" => kind("interval"),
        "TIMESTAMP_S" => timestamp("s", false),
        "TIMESTAMP_MS" => timestamp("ms", false),
        "TIMESTAMP" => timestamp("us", false),
        "TIMESTAMP_NS" => timestamp("ns", false),
        "TIMESTAMP WITH TIME ZONE" | "TIMESTAMPTZ" => timestamp("us", true),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs_4_4_maps_duckdb_types_as_the_cat_4_4_table_states() {
        assert_eq!(
            logical_type("INTEGER"),
            json!({"kind": "integer", "bits": 32, "signed": true})
        );
        assert_eq!(
            logical_type("UBIGINT"),
            json!({"kind": "integer", "bits": 64, "signed": false})
        );
        assert_eq!(
            logical_type("DECIMAL(7,2)"),
            json!({"kind": "decimal", "precision": 7, "scale": 2})
        );
        assert_eq!(logical_type("VARCHAR"), json!({"kind": "text"}));
        assert_eq!(logical_type("DATE"), json!({"kind": "date"}));
        assert_eq!(
            logical_type("TIMESTAMP WITH TIME ZONE"),
            json!({"kind": "timestamp", "precision": "us", "with_time_zone": true})
        );
        assert_eq!(
            logical_type("VARCHAR[]"),
            json!({"kind": "list", "element": {"kind": "text"}})
        );
    }

    #[test]
    fn cs_4_4_maps_every_type_the_table_does_not_list_to_unsupported() {
        for name in [
            "JSON",
            "BIT",
            "TIMETZ",
            "STRUCT(a INTEGER)",
            "MAP(VARCHAR, INTEGER)",
        ] {
            assert_eq!(logical_type(name), json!({"kind": "unsupported"}), "{name}");
        }
        assert_eq!(logical_type("JSON[]"), json!({"kind": "unsupported"}));
    }

    #[test]
    fn cs_4_4_describes_every_created_table() {
        let relations = [CreatedRelation {
            source: "scott.emp".to_owned(),
            columns: vec![CreatedColumn {
                name: "sal".to_owned(),
                physical_type: "DECIMAL(7,2)".to_owned(),
                nullable: false,
            }],
        }];
        let document = catalog_schema_document("query/a01_grand_total", "v2.0.0", &relations);
        assert_eq!(document["contract"], "menura.catalog_schema");
        assert_eq!(document["relations"][0]["source"], "scott.emp");
        assert_eq!(
            document["relations"][0]["columns"][0],
            json!({
                "name": "sal",
                "logical_type": {"kind": "decimal", "precision": 7, "scale": 2},
                "physical_type": "DECIMAL(7,2)",
                "nullable": false,
            })
        );
    }
}
