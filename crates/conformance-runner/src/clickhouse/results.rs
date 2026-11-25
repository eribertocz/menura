//! Reads the `TabSeparated` rows `ClickHouse` returns and reduces each value to the cell CS-2.4
//! compares, by the logical type of its column.

use menura_contracts::types::common::LogicalType;
use menura_contracts::types::compiled_plan::ResultColumn;

use crate::calendar::{format_timestamp, parse_timestamp};
use crate::cells::{Cell, expected_cell};

/// The cells of the `TabSeparated` text `output`, one row per line, read by `schema`.
///
/// # Errors
///
/// Why a row or a value cannot be read.
pub(crate) fn result_cells(
    schema: &[ResultColumn],
    output: &str,
) -> Result<Vec<Vec<Cell>>, String> {
    output
        .split_terminator('\n')
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != schema.len() {
                return Err(format!(
                    "ClickHouse returns rows of {} columns, but the result schema has {}",
                    fields.len(),
                    schema.len()
                ));
            }
            fields
                .iter()
                .zip(schema)
                .map(|(field, column)| cell(field, &column.logical_type))
                .collect()
        })
        .collect()
}

/// The cell of one escaped field in a column of type `logical_type`.
fn cell(field: &str, logical_type: &LogicalType) -> Result<Cell, String> {
    if field == "\\N" {
        return Ok(Cell::Null);
    }
    let text = unescape(field);
    match logical_type {
        LogicalType::Timestamp { with_time_zone, .. } => {
            let iso = format!(
                "{}{}",
                text.replacen(' ', "T", 1),
                if *with_time_zone { "Z" } else { "" }
            );
            parse_timestamp(&iso, *with_time_zone)
                .map(|micros| Cell::Text(format_timestamp(micros, *with_time_zone)))
                .ok_or_else(|| format!("`{text}` is not a ClickHouse timestamp"))
        }
        _ => expected_cell(Some(&text), logical_type),
    }
}

/// `field` without the escapes of `ClickHouse`'s escaped text form.
pub(crate) fn unescape(field: &str) -> String {
    let mut text = String::with_capacity(field.len());
    let mut characters = field.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            text.push(character);
            continue;
        }
        match characters.next() {
            Some('t') => text.push('\t'),
            Some('n') => text.push('\n'),
            Some('r') => text.push('\r'),
            Some('0') => text.push('\0'),
            Some('b') => text.push('\u{8}'),
            Some('f') => text.push('\u{c}'),
            Some(other) => text.push(other),
            None => text.push('\\'),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::{IntegerBits, TimestampPrecision};
    use menura_contracts::types::compiled_plan::ColumnRole;

    use super::*;

    fn column(logical_type: LogicalType) -> ResultColumn {
        ResultColumn {
            name: "c".to_owned(),
            role: ColumnRole::Dimension,
            logical_type,
            physical_type: String::new(),
            nullable: true,
        }
    }

    #[test]
    fn cs_2_4_reads_null_numbers_and_escaped_text() {
        let schema = [
            column(LogicalType::Integer {
                bits: IntegerBits::SixtyFour,
                signed: true,
            }),
            column(LogicalType::Text),
        ];
        let cells = result_cells(&schema, "45\ta\\tb\n\\N\t\\N\n").unwrap();
        assert_eq!(cells.len(), 2);
        assert_eq!(
            cells[0][0],
            expected_cell(Some("45.00"), &schema[0].logical_type).unwrap()
        );
        assert_eq!(cells[0][1], Cell::Text("a\tb".to_owned()));
        assert_eq!(cells[1], vec![Cell::Null, Cell::Null]);
    }

    #[test]
    fn cs_2_4_reads_an_instant_in_its_con_4_form() {
        let schema = [column(LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: true,
        })];
        let cells = result_cells(&schema, "2026-01-05 10:00:00.500000\n").unwrap();
        assert_eq!(cells[0][0], Cell::Text("2026-01-05T10:00:00.5Z".to_owned()));
    }

    #[test]
    fn cs_2_4_an_empty_output_has_no_rows() {
        assert!(
            result_cells(&[column(LogicalType::Text)], "")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn cs_2_4_refuses_a_row_of_another_width() {
        assert!(result_cells(&[column(LogicalType::Text)], "a\tb\n").is_err());
    }
}
