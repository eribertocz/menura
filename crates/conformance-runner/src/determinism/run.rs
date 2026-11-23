//! One run of the determinism check: every wrapper call of the suite computed through `core`,
//! its documents written to a file; and the comparison of two such files (PLN-2.4, P3).

use std::fmt;
use std::path::Path;

use serde_json::{Value, json};

use super::document::reorder_document;
use super::input_order::equal_up_to_input_order;
use super::permutation::{Permutation, SEED};
use crate::wrappers::{
    CallValue, Expected, ParityError, WrapperCall, calls_of_cases, expected_result,
    first_difference, usage_probes,
};

/// The documents one call gave in one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallOutput {
    /// The case of the call.
    pub case: String,
    /// The command of the call.
    pub command: String,
    /// The output document, or the usage error.
    pub document: String,
    /// The audit record, when the call gave one.
    pub audit_record: Option<String>,
}

/// What one run gave: every call's documents, and how many input documents it reordered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutputs {
    /// The documents of every call, in call order.
    pub calls: Vec<CallOutput>,
    /// The input documents the run reordered.
    pub reordered: usize,
    /// The input documents the run passed unchanged because they cannot be reordered safely,
    /// as `case: argument`, each distinct one once.
    pub kept: Vec<String>,
}

/// Computes every call of the suite in `conformance_directory`; when `reorder` is true, every
/// input document first gets its mappings reordered with the fixed seed.
///
/// # Errors
///
/// Returns a [`ParityError`] when a case cannot be read or `core` cannot write a document.
pub fn run_calls(conformance_directory: &Path, reorder: bool) -> Result<RunOutputs, ParityError> {
    let output_error = |error: crate::RunnerError| ParityError::Output {
        case: conformance_directory.display().to_string(),
        reason: error.to_string(),
    };
    let mut calls = calls_of_cases(conformance_directory).map_err(output_error)?;
    calls.extend(usage_probes(conformance_directory).map_err(output_error)?);
    let mut permutation = Permutation::new(SEED);
    let mut outputs = RunOutputs {
        calls: Vec::with_capacity(calls.len()),
        reordered: 0,
        kept: Vec::new(),
    };
    for mut call in calls {
        if reorder {
            reorder_call(&mut call, &mut permutation, &mut outputs);
        }
        outputs
            .calls
            .push(call_output(&call, expected_result(&call)?));
    }
    Ok(outputs)
}

/// Reorders every document argument of `call` that can be reordered safely.
fn reorder_call(call: &mut WrapperCall, permutation: &mut Permutation, outputs: &mut RunOutputs) {
    for (argument, value) in &mut call.arguments {
        if let CallValue::Document(document) = value {
            if let Some(bytes) = reorder_document(&document.bytes, permutation) {
                document.bytes = bytes;
                outputs.reordered += 1;
            } else {
                let kept = format!("{}: {}", call.case, argument.name());
                if !outputs.kept.contains(&kept) {
                    outputs.kept.push(kept);
                }
            }
        }
    }
}

fn call_output(call: &WrapperCall, expected: Expected) -> CallOutput {
    let (document, audit_record) = match expected {
        Expected::Output(output) => (output.document, output.audit_record),
        Expected::Usage(error) => (format!("usage error: {error}"), None),
    };
    CallOutput {
        case: call.case.clone(),
        command: call.command.name().to_owned(),
        document,
        audit_record,
    }
}

impl RunOutputs {
    /// The outputs as a JSON document, to pass from a child process to the parent.
    #[must_use]
    pub fn to_json(&self) -> String {
        let calls: Vec<Value> = self
            .calls
            .iter()
            .map(|call| {
                json!({
                    "case": call.case,
                    "command": call.command,
                    "document": call.document,
                    "audit_record": call.audit_record,
                })
            })
            .collect();
        json!({"calls": calls, "reordered": self.reordered, "kept": self.kept}).to_string()
    }

    /// The outputs written by [`RunOutputs::to_json`], or `None` when `text` is not such a
    /// document.
    #[must_use]
    pub fn from_json(text: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(text).ok()?;
        let count = |name: &str| {
            value
                .get(name)
                .and_then(Value::as_u64)
                .and_then(|count| usize::try_from(count).ok())
        };
        let text_of = |call: &Value, name: &str| call.get(name)?.as_str().map(str::to_owned);
        let calls = value
            .get("calls")?
            .as_array()?
            .iter()
            .map(|call| {
                Some(CallOutput {
                    case: text_of(call, "case")?,
                    command: text_of(call, "command")?,
                    document: text_of(call, "document")?,
                    audit_record: text_of(call, "audit_record"),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            calls,
            reordered: count("reordered")?,
            kept: value
                .get("kept")?
                .as_array()?
                .iter()
                .map(|kept| kept.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?,
        })
    }
}

/// A difference between the first run and the reordered run of one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDifference {
    /// The case of the call.
    pub case: String,
    /// The command of the call.
    pub command: String,
    /// The run that differs from the first, and what differs.
    pub detail: String,
}

impl fmt::Display for RunDifference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "case {}: `{}`: run 2 (reordered keys, seed {SEED:#x}) differs from run 1: {}",
            self.case, self.command, self.detail
        )
    }
}

/// The comparison of two runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// The calls whose documents differ.
    pub differences: Vec<RunDifference>,
    /// The calls whose documents differ only in what the specification defines from the input text
    /// ([`equal_up_to_input_order`]).
    pub input_order_only: usize,
}

/// Compares the documents of every call of `first` and `second` byte for byte. A document that
/// differs only in what the specification defines from the input text is counted apart; an audit
/// record must be byte-identical.
#[must_use]
pub fn compare_runs(first: &RunOutputs, second: &RunOutputs) -> Comparison {
    let mut comparison = Comparison {
        differences: Vec::new(),
        input_order_only: 0,
    };
    if first.calls.len() != second.calls.len() {
        comparison.differences.push(RunDifference {
            case: "*".to_owned(),
            command: "*".to_owned(),
            detail: format!(
                "{} calls in run 1, {} in run 2",
                first.calls.len(),
                second.calls.len()
            ),
        });
    }
    for (left, right) in first.calls.iter().zip(&second.calls) {
        match compare_call(left, right) {
            CallComparison::Identical => {}
            CallComparison::InputOrderOnly => comparison.input_order_only += 1,
            CallComparison::Different(detail) => comparison.differences.push(RunDifference {
                case: left.case.clone(),
                command: left.command.clone(),
                detail,
            }),
        }
    }
    comparison
}

/// How the documents of one call compare.
enum CallComparison {
    Identical,
    InputOrderOnly,
    Different(String),
}

fn compare_call(left: &CallOutput, right: &CallOutput) -> CallComparison {
    if left.case != right.case || left.command != right.command {
        return CallComparison::Different(format!(
            "run 2 gave call `{}` `{}`",
            right.case, right.command
        ));
    }
    let audit = |call: &CallOutput| call.audit_record.clone().unwrap_or_default();
    if let Some(difference) = first_difference(&audit(left), &audit(right)) {
        return CallComparison::Different(format!("audit record: {difference}"));
    }
    match first_difference(&left.document, &right.document) {
        None => CallComparison::Identical,
        Some(_) if equal_up_to_input_order(&left.document, &right.document) => {
            CallComparison::InputOrderOnly
        }
        Some(difference) => CallComparison::Different(format!("document: {difference}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outputs(document: &str, audit_record: Option<&str>) -> RunOutputs {
        RunOutputs {
            calls: vec![CallOutput {
                case: "e3-sample-001".to_owned(),
                command: "compile".to_owned(),
                document: document.to_owned(),
                audit_record: audit_record.map(str::to_owned),
            }],
            reordered: 1,
            kept: vec!["e3-sample-001: query".to_owned()],
        }
    }

    #[test]
    fn pln_2_4_an_injected_difference_is_reported_with_its_case_run_and_line() {
        let first = outputs("{\n  \"cache_key\": \"a\"\n}", None);
        let second = outputs("{\n  \"cache_key\": \"b\"\n}", None);
        let differences = compare_runs(&first, &second).differences;
        assert_eq!(differences.len(), 1);
        assert_eq!(
            differences[0].to_string(),
            format!(
                "case e3-sample-001: `compile`: run 2 (reordered keys, seed {SEED:#x}) differs \
                 from run 1: document: line 2, column 17: expected `  \"cache_key\": \"a\"`, \
                 found `  \"cache_key\": \"b\"`"
            )
        );
    }

    #[test]
    fn pln_2_4_a_plan_that_differs_only_in_member_order_is_reported() {
        let plan = "{\n  \"contract\": \"menura.compiled_plan\",\n  \"sql\": \"SELECT 1\"\n}";
        let reordered = "{\n  \"sql\": \"SELECT 1\",\n  \"contract\": \"menura.compiled_plan\"\n}";
        let comparison = compare_runs(&outputs(plan, None), &outputs(reordered, None));
        assert_eq!(comparison.input_order_only, 0);
        assert_eq!(comparison.differences.len(), 1);
        assert!(
            comparison.differences[0]
                .detail
                .starts_with("document: line 2")
        );
    }

    #[test]
    fn pln_2_4_an_injected_audit_difference_is_reported() {
        let first = outputs("{}", Some("{\"hash\": 1}"));
        let second = outputs("{}", Some("{\"hash\": 2}"));
        let differences = compare_runs(&first, &second).differences;
        assert!(differences[0].detail.starts_with("audit record: line 1"));
    }

    #[test]
    fn pln_2_4_identical_runs_have_no_difference() {
        let first = outputs("{}", Some("{}"));
        assert!(compare_runs(&first, &first.clone()).differences.is_empty());
        let missing = RunOutputs {
            calls: Vec::new(),
            ..first.clone()
        };
        assert_eq!(compare_runs(&first, &missing).differences.len(), 1);
    }

    #[test]
    fn pln_2_4_a_text_position_is_counted_apart() {
        let first = outputs("{\"location\": {\"pointer\": \"/a\", \"line\": 2}}", None);
        let second = outputs("{\"location\": {\"pointer\": \"/a\", \"line\": 5}}", None);
        let comparison = compare_runs(&first, &second);
        assert!(comparison.differences.is_empty());
        assert_eq!(comparison.input_order_only, 1);
    }

    #[test]
    fn pln_2_4_outputs_pass_between_processes_unchanged() {
        let first = outputs("{\n  \"a\": \"\\u00e9\"\n}", Some("{}"));
        assert_eq!(RunOutputs::from_json(&first.to_json()), Some(first));
        assert_eq!(RunOutputs::from_json("[]"), None);
    }
}
