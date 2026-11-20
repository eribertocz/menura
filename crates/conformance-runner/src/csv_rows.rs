//! Reads the expected `rows` file of a case (CS-2.4): comma-separated values with a header line,
//! quoted as RFC 4180 says. An empty field without quotes is NULL; `""` is the empty text.

/// One field of a `rows` file: `None` is NULL.
pub(crate) type CsvField = Option<String>;

/// The content of a `rows` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CsvRows {
    /// The output names of the header line, in order.
    pub(crate) header: Vec<String>,
    /// The rows after the header line.
    pub(crate) rows: Vec<Vec<CsvField>>,
}

/// Parses the text of a `rows` file.
///
/// Lines end with `\n` or `\r\n`; the line break after the last line is optional.
///
/// # Errors
///
/// Returns why the text is not such a file: no header line, a NULL output name, a quote that
/// is not closed, or a quoted field followed by other characters.
pub(crate) fn parse_csv(text: &str) -> Result<CsvRows, String> {
    let mut lines = split_records(text)?.into_iter();
    let header = lines
        .next()
        .ok_or_else(|| "the rows file has no header line".to_owned())?
        .into_iter()
        .map(|name| name.ok_or_else(|| "an output name of the header line is empty".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CsvRows {
        header,
        rows: lines.collect(),
    })
}

/// Splits the text into records of fields.
fn split_records(text: &str) -> Result<Vec<Vec<CsvField>>, String> {
    let mut records = Vec::new();
    let mut characters = text.chars().peekable();
    while characters.peek().is_some() {
        let mut record = Vec::new();
        loop {
            let (field, end) = read_field(&mut characters)?;
            record.push(field);
            if end != FieldEnd::Comma {
                break;
            }
        }
        records.push(record);
    }
    Ok(records)
}

/// What follows a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldEnd {
    Comma,
    LineEnd,
}

/// Reads one field and the character that ends it.
fn read_field(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<(CsvField, FieldEnd), String> {
    if characters.peek() == Some(&'"') {
        characters.next();
        let field = read_quoted(characters)?;
        return match read_end(characters) {
            Some(end) => Ok((Some(field), end)),
            None => Err("a quoted field is followed by other characters".to_owned()),
        };
    }
    let mut field = String::new();
    loop {
        match characters.next() {
            None | Some('\n') => return Ok((non_empty(field), FieldEnd::LineEnd)),
            Some(',') => return Ok((non_empty(field), FieldEnd::Comma)),
            Some('\r') if characters.peek() == Some(&'\n') => {}
            Some(character) => field.push(character),
        }
    }
}

/// Reads a quoted field after its opening quote, up to and including its closing quote.
fn read_quoted(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String, String> {
    let mut field = String::new();
    loop {
        match characters.next() {
            None => return Err("a quoted field is not closed".to_owned()),
            Some('"') if characters.peek() == Some(&'"') => {
                characters.next();
                field.push('"');
            }
            Some('"') => return Ok(field),
            Some(character) => field.push(character),
        }
    }
}

/// Reads the end of a quoted field, or `None` when another character follows it.
fn read_end(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<FieldEnd> {
    match characters.next() {
        None | Some('\n') => Some(FieldEnd::LineEnd),
        Some(',') => Some(FieldEnd::Comma),
        Some('\r') if characters.next() == Some('\n') => Some(FieldEnd::LineEnd),
        Some(_) => None,
    }
}

fn non_empty(field: String) -> CsvField {
    (!field.is_empty()).then_some(field)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> String {
        value.to_owned()
    }

    #[test]
    fn cs_2_4a_reads_the_header_and_the_rows() {
        let rows = parse_csv("region,total\nNORTH,1.50\nSOUTH,2\n").unwrap();
        assert_eq!(rows.header, ["region", "total"]);
        assert_eq!(
            rows.rows,
            [
                vec![Some(text("NORTH")), Some(text("1.50"))],
                vec![Some(text("SOUTH")), Some(text("2"))]
            ]
        );
    }

    #[test]
    fn cs_2_4a_an_empty_unquoted_field_is_null_and_a_quoted_one_is_empty_text() {
        let rows = parse_csv("a,b,c\n,\"\",x\r\n").unwrap();
        assert_eq!(rows.rows, [vec![None, Some(text("")), Some(text("x"))]]);
    }

    #[test]
    fn cs_2_4a_a_quoted_field_keeps_commas_quotes_and_line_breaks() {
        let rows = parse_csv("a\n\"x, \"\"y\"\"\nz\"").unwrap();
        assert_eq!(rows.rows, [vec![Some(text("x, \"y\"\nz"))]]);
    }

    #[test]
    fn cs_2_4a_a_header_without_rows_expects_no_rows() {
        let rows = parse_csv("region,sale_count\n").unwrap();
        assert!(rows.rows.is_empty());
    }

    #[test]
    fn cs_2_4a_rejects_a_file_without_header_or_with_an_open_quote() {
        assert!(parse_csv("").is_err());
        assert!(parse_csv("a,\n").is_err());
        assert!(parse_csv("a\n\"x\n").is_err());
        assert!(parse_csv("a\n\"x\"y\n").is_err());
    }
}
