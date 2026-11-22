//! The errors of E4: `ESEC-02`, `EQRY-03`, and the internal errors of an input the stages
//! before E4 never produce.

use std::fmt;

use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_representations::origin::{Origin, QueryLocation};
use menura_security::RowFilterError;

/// The message of `EQRY-03` in E4: it names neither the policy nor its dataset (SCY-4a.1).
const MISSING_REFERENCE_TIME: &str =
    "the query needs `reference_time` to apply the security rules; add it to the query";

/// The message of `E4-03` (ADD-4.9).
const SPLIT_SNAPSHOT: &str = "this grain is finer than the `snapshot_grain` of a semi-additive \
    metric, whose snapshots cannot be split; group by its `snapshot_grain` or a coarser grain";

/// The message of `E4-02` (E4-8.6, CTX-3.1).
const CONTEXT_TIME: &str = "this metric needs its `time_dimension` in the query exactly once, \
    at a grain equal to or finer than its `grain`, with a time grain when it is a timestamp";

/// Why E4 stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// `ESEC-02`: an applicable row policy needs an attribute the security context does not
    /// provide (E4-6.9, E2-5 row 18).
    MissingAttribute(RowFilterError),
    /// `EQRY-03`: an applicable row policy reads the current date or time and the query has no
    /// `reference_time` (E4-6.9b, QRY-2). It carries nothing about the policy, so that no
    /// message can name it (SCY-4a.1).
    MissingReferenceTime,
    /// `E4-03`: the query groups by the `over` field of a semi-additive metric at a grain
    /// finer than its `snapshot_grain` (ADD-4.9, E4-7.8, E2-5 row 19), located at the
    /// `time_grains` entry, else the dimension, else the SQL `SELECT` item.
    SplitSnapshot(QueryLocation),
    /// `E4-02`: a context modifier needs a time dimension the query lacks, at a coarser grain,
    /// or a `timestamp` without a time grain (E4-8.6, CTX-3.1), located at the query metric
    /// that reads the context metric.
    ContextTime(QueryLocation),
    /// The input breaks the contract of the stages before E4. This is a defect of the
    /// compiler, not a query error, so it has no specification code.
    Invariant {
        /// What the input breaks.
        violation: InvariantViolation,
        /// The input element it is found at.
        origin: Origin,
    },
}

impl PlanError {
    /// The diagnostic of the compile error document (PLN-5): `ESEC-02` at `/attributes` of
    /// the security context, `EQRY-03` at `/query` of the query; `None` for an internal error.
    #[must_use]
    pub fn to_diagnostic(&self) -> Option<Diagnostic> {
        match self {
            Self::MissingAttribute(error) => error.to_diagnostic(),
            Self::MissingReferenceTime => Some(Diagnostic {
                code: "EQRY-03".to_owned(),
                message: MISSING_REFERENCE_TIME.to_owned(),
                location: Some(DiagnosticLocation {
                    document: InputDocument::Query,
                    pointer: "/query".to_owned(),
                    line: None,
                    column: None,
                }),
                alternatives: None,
            }),
            Self::SplitSnapshot(location) => {
                Some(query_diagnostic("E4-03", SPLIT_SNAPSHOT, location))
            }
            Self::ContextTime(location) => Some(query_diagnostic("E4-02", CONTEXT_TIME, location)),
            Self::Invariant { .. } => None,
        }
    }

    /// The internal error `violation` at `origin`.
    pub(crate) fn invariant(violation: InvariantViolation, origin: &Origin) -> Self {
        Self::Invariant {
            violation,
            origin: origin.clone(),
        }
    }
}

/// The diagnostic `code` with `message`, located at `location` of the query.
fn query_diagnostic(code: &str, message: &str, location: &QueryLocation) -> Diagnostic {
    let position = location.position();
    Diagnostic {
        code: code.to_owned(),
        message: message.to_owned(),
        location: Some(DiagnosticLocation {
            document: InputDocument::Query,
            pointer: location.pointer().to_owned(),
            line: position.and_then(|position| u64::try_from(position.line()).ok()),
            column: position.and_then(|position| u64::try_from(position.column()).ok()),
        }),
        alternatives: None,
    }
}

/// An input that the stages before E4 never produce, or that needs a later part of E4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantViolation {
    /// The query has no branch.
    NoBranch,
    /// Two branches whose `Aggregate` keys differ, or a branch that does not end with its
    /// `Aggregate`, so that they cannot be stitched (E4-5.0a).
    UnstitchableBranches,
    /// A join instance whose relationship the model does not have, or does not match its
    /// step, its parent instance or its column pairs (E3-4.2a); or a relationship of the
    /// usable graph that the model does not have.
    UnknownRelationship,
    /// A scanned dataset that E0 marks unusable, which E2 rules out (E0-4.2a, E2-5 row 12a).
    UnusableDataset,
    /// A scanned dataset with row policies whose row filter was not computed (E4-6.9).
    UnvisitedPolicies,
    /// The home dataset is not in the model.
    UnknownDataset,
    /// A field that is not in the model, or of another dataset than the join instance that
    /// reads it.
    ForeignField,
    /// A field without a type, or whose type is the type of `NULL`.
    UntypedField,
    /// An expression the plan cannot type, which E0.f typed.
    UntypedExpression,
    /// A metric the model does not have, or that E0.g did not classify.
    UnknownMetric,
    /// A component key that names no component of its metric.
    UnknownComponent,
    /// A metric whose aggregates do not match its components (E0G-1.1).
    ComponentMismatch,
    /// A derived metric whose inputs lead back to it, which E0 reports as `E0C-34`
    /// (DER-5.1.4).
    DerivedCycle,
    /// A population that no dataset of the model declares (POP-2).
    UnknownPopulation,
    /// A unit of periods that does not output the period column it is read by (E4-3.12).
    UnknownPeriodColumn,
    /// A context metric whose modifier E4 does not plan yet (`period_to_date`),
    /// or one that the base of another context metric reads (`E0C-64`).
    ContextMetric,
    /// Context queries other than those the query's context metrics need, in the order of
    /// E4-8.8 (E4-8.2).
    UnplannedContextQuery,
    /// A home dataset whose `source` the catalog schema does not describe.
    UnknownRelation,
    /// A source column, read by a dataset metric, that the catalog relation does not have
    /// (E0D-3).
    UnknownSourceColumn,
    /// A row policy whose condition E0 did not parse.
    UnparsedCondition,
    /// A date range on a field that is neither a date nor a timestamp.
    UnplannableDateRange,
    /// An ordering term on a column the query does not output.
    UnknownOrderColumn,
    /// A column name that is empty.
    EmptyName,
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAttribute(error) => error.fmt(formatter),
            Self::MissingReferenceTime => write!(formatter, "EQRY-03: {MISSING_REFERENCE_TIME}"),
            Self::SplitSnapshot(location) => {
                write!(
                    formatter,
                    "E4-03: {SPLIT_SNAPSHOT} at {}",
                    location.pointer()
                )
            }
            Self::ContextTime(location) => {
                write!(formatter, "E4-02: {CONTEXT_TIME} at {}", location.pointer())
            }
            Self::Invariant { violation, origin } => {
                let pointer = match origin {
                    Origin::Query(location) => location.pointer(),
                    Origin::Model(location) => location.pointer.as_str(),
                };
                write!(formatter, "internal error {violation:?} at {pointer}")
            }
        }
    }
}

impl std::error::Error for PlanError {}
