//! The entries of the explanation, as each stage records its decisions (PLN-7).

use std::fmt;

use menura_contracts::types::common::{Decision, Step};
use menura_model::internal_model::{FieldPath, NormalizedName};

/// The object a decision is about (PLN-7.2), written `<kind>:<name>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Subject {
    /// `dataset:<name>`.
    Dataset(NormalizedName),
    /// `field:<dataset>.<field>`.
    Field(FieldPath),
    /// `relationship:<name>`.
    Relationship(NormalizedName),
    /// `policy:<dataset>.<policy>`: a row policy, by the dataset that declares it.
    Policy {
        /// The dataset that declares the policy.
        dataset: NormalizedName,
        /// The policy's name.
        policy: String,
    },
    /// `summary:<name>`.
    Summary(NormalizedName),
    /// `path:<from>-><to>`: a `paths` entry of the query.
    Path {
        /// The dataset the path starts from.
        from: NormalizedName,
        /// The dataset the path reaches.
        to: NormalizedName,
    },
    /// `role:<role>`, as the security context spells it.
    Role(String),
    /// `attribute:<name>`, as the security context spells it.
    Attribute(String),
    /// `load_option:<name>`.
    LoadOption(&'static str),
}

impl fmt::Display for Subject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dataset(name) => write!(formatter, "dataset:{}", name.as_str()),
            Self::Field(field) => write!(
                formatter,
                "field:{}.{}",
                field.dataset.as_str(),
                field.field.as_str()
            ),
            Self::Relationship(name) => write!(formatter, "relationship:{}", name.as_str()),
            Self::Policy { dataset, policy } => {
                write!(formatter, "policy:{}.{policy}", dataset.as_str())
            }
            Self::Summary(name) => write!(formatter, "summary:{}", name.as_str()),
            Self::Path { from, to } => {
                write!(formatter, "path:{}->{}", from.as_str(), to.as_str())
            }
            Self::Role(role) => write!(formatter, "role:{role}"),
            Self::Attribute(name) => write!(formatter, "attribute:{name}"),
            Self::LoadOption(name) => write!(formatter, "load_option:{name}"),
        }
    }
}

/// One decision a stage records for the explanation (PLN-7).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExplanationEntry {
    /// The rule ID that decides it, such as `E5-3.2` (PLN-7.3).
    pub code: &'static str,
    /// The stage that decides it.
    pub step: Step,
    /// The object decided about.
    pub subject: Subject,
    /// A short statement of the decision; `applied` or `rejected` for an E5-3 rewrite.
    pub decision: String,
    /// Why, naming the condition that held or failed; never a value (PLN-4.2).
    pub reason: String,
}

impl ExplanationEntry {
    /// The entry as the compiled plan writes it (PLN-7).
    ///
    /// # Example
    ///
    /// ```
    /// use menura_contracts::types::common::Step;
    /// use menura_representations::explanation::{ExplanationEntry, Subject};
    ///
    /// let entry = ExplanationEntry {
    ///     code: "SEC-3.2",
    ///     step: Step::E2,
    ///     subject: Subject::Role("auditor".to_owned()),
    ///     decision: "role ignored".to_owned(),
    ///     reason: "the model does not declare it".to_owned(),
    /// };
    /// assert_eq!(entry.to_decision().subject.as_deref(), Some("role:auditor"));
    /// ```
    #[must_use]
    pub fn to_decision(&self) -> Decision {
        Decision {
            code: Some(self.code.to_owned()),
            step: self.step,
            subject: Some(self.subject.to_string()),
            decision: self.decision.clone(),
            reason: self.reason.clone(),
        }
    }
}
