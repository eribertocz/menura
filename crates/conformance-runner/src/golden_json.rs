//! Golden JSON files (CS-2.10): read in their stored form and compared with a produced value
//! after RFC 8785 canonicalization.

use std::path::Path;

use crate::canonical_json::JsonValue;
use crate::fixture::read_text;

/// Reads the golden JSON file `file_name` next to the case. It fails, naming the file, when the
/// file cannot be read, is not valid JSON, has a duplicate member name, or is not in stored form
/// (CS-2.10).
pub(crate) fn read_golden(case_folder: &Path, file_name: &str) -> Result<JsonValue, String> {
    let text = read_text(&case_folder.join(file_name)).map_err(|error| error.to_string())?;
    let golden = JsonValue::parse(&text)
        .map_err(|reason| format!("the golden file `{file_name}` is not valid JSON: {reason}"))?;
    if golden.stored_form() != text {
        return Err(format!(
            "the golden file `{file_name}` is not in stored form (CS-2.10)"
        ));
    }
    Ok(golden)
}

/// Why `produced` differs from `golden` once both are canonical JSON, or `None` (CS-2.10).
pub(crate) fn json_mismatch(
    golden: &JsonValue,
    produced: &JsonValue,
    file_name: &str,
) -> Option<String> {
    (golden.canonical() != produced.canonical()).then(|| {
        format!(
            "the output differs from the golden file `{file_name}`; the output is:\n{}",
            produced.stored_form()
        )
    })
}

/// Why the produced value differs from the golden file `file_name` next to the case, or why
/// the golden file is not valid, or `None` (CS-2.10).
pub(crate) fn golden_mismatch(
    case_folder: &Path,
    file_name: &str,
    produced: &JsonValue,
) -> Option<String> {
    match read_golden(case_folder, file_name) {
        Ok(golden) => json_mismatch(&golden, produced, file_name),
        Err(reason) => Some(reason),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn case_folder() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases/cs")
    }

    fn produced(text: &str) -> JsonValue {
        JsonValue::parse(text).unwrap()
    }

    #[test]
    fn cs_2_10_a_value_equal_after_canonicalization_matches() {
        let value = produced(r#"{"b": 1.0, "a": [true]}"#);
        assert_eq!(
            golden_mismatch(&case_folder(), "cs-unit-golden-001.json", &value),
            None
        );
    }

    #[test]
    fn cs_2_10_a_different_value_fails_and_shows_the_output() {
        let value = produced(r#"{"a": [false], "b": 1}"#);
        let reason = golden_mismatch(&case_folder(), "cs-unit-golden-001.json", &value).unwrap();
        assert!(reason.contains("cs-unit-golden-001.json"), "{reason}");
        assert!(reason.contains("false"), "{reason}");
    }

    #[test]
    fn cs_2_10_a_golden_file_not_in_stored_form_fails_naming_it() {
        let reason = read_golden(&case_folder(), "cs-unit-golden-unsorted-001.json").unwrap_err();
        assert!(
            reason.contains("cs-unit-golden-unsorted-001.json"),
            "{reason}"
        );
        assert!(reason.contains("stored form"), "{reason}");
    }

    #[test]
    fn cs_2_10_a_golden_file_with_a_duplicate_member_fails() {
        let reason = read_golden(&case_folder(), "cs-unit-golden-duplicate-001.json").unwrap_err();
        assert!(reason.contains("repeated"), "{reason}");
    }

    #[test]
    fn cs_2_10_a_golden_file_that_is_not_json_fails() {
        let reason = read_golden(&case_folder(), "cs-unit-golden-invalid-001.json").unwrap_err();
        assert!(reason.contains("not valid JSON"), "{reason}");
    }

    #[test]
    fn cs_2_10_a_missing_golden_file_fails() {
        assert!(read_golden(&case_folder(), "cs-unit-golden-missing-001.json").is_err());
    }
}
