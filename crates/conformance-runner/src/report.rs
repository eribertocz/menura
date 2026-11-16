//! The conformance report: the result of every case, grouped per rule ID (CS-4.3).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::case::{CaseId, RuleId};

/// The result of one case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseStatus {
    /// The stage the case needs is not implemented yet; the case is neither passed nor failed.
    Pending(String),
    /// The case failed, with the reason.
    Failed(String),
}

impl CaseStatus {
    fn label(&self) -> &'static str {
        match self {
            Self::Pending(_) => "pending",
            Self::Failed(_) => "failed",
        }
    }

    fn reason(&self) -> &str {
        match self {
            Self::Pending(reason) | Self::Failed(reason) => reason,
        }
    }
}

/// One case and its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseResult {
    /// The case identifier.
    pub id: CaseId,
    /// The rule IDs the case covers.
    pub rules: Vec<RuleId>,
    /// The result.
    pub status: CaseStatus,
}

/// The results of a run, in the order the cases were run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Every case that was run.
    pub cases: Vec<CaseResult>,
}

impl Report {
    /// The cases covering each rule ID, rule IDs sorted (CS-4.3).
    #[must_use]
    pub fn by_rule(&self) -> BTreeMap<&RuleId, Vec<&CaseResult>> {
        let mut rules: BTreeMap<&RuleId, Vec<&CaseResult>> = BTreeMap::new();
        for case in &self.cases {
            for rule in &case.rules {
                rules.entry(rule).or_default().push(case);
            }
        }
        rules
    }

    /// Whether any case failed.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.cases
            .iter()
            .any(|case| matches!(case.status, CaseStatus::Failed(_)))
    }

    /// The report as text: each rule ID, then each of its cases with its result and reason,
    /// then a summary line.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::new();
        for (rule, cases) in self.by_rule() {
            let _ = writeln!(text, "{rule}");
            for case in cases {
                let _ = writeln!(
                    text,
                    "  {} {}: {}",
                    case.id,
                    case.status.label(),
                    case.status.reason()
                );
            }
        }
        let failed = self
            .cases
            .iter()
            .filter(|case| matches!(case.status, CaseStatus::Failed(_)))
            .count();
        let _ = writeln!(
            text,
            "{} cases: {} pending, {failed} failed",
            self.cases.len(),
            self.cases.len() - failed
        );
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Header {
        id: CaseId,
        rules: Vec<RuleId>,
    }

    fn case(id: &str, rules: &[&str], status: CaseStatus) -> CaseResult {
        let text = format!("id: {id}\nrules: [{}]", rules.join(", "));
        let header: Header = serde_saphyr::from_str(&text).unwrap();
        CaseResult {
            id: header.id,
            rules: header.rules,
            status,
        }
    }

    #[test]
    fn cs_4_3_reports_every_case_under_each_rule_it_covers() {
        let report = Report {
            cases: vec![
                case("b-001", &["X-2", "X-1"], CaseStatus::Pending("p".into())),
                case("a-001", &["X-1"], CaseStatus::Failed("f".into())),
            ],
        };
        assert_eq!(
            report.render(),
            "X-1\n  b-001 pending: p\n  a-001 failed: f\nX-2\n  b-001 pending: p\n\
             2 cases: 1 pending, 1 failed\n"
        );
        assert!(report.has_failures());
    }

    #[test]
    fn cs_4_3_pending_is_not_a_failure() {
        let report = Report {
            cases: vec![case("a-001", &["X-1"], CaseStatus::Pending("p".into()))],
        };
        assert!(!report.has_failures());
    }
}
