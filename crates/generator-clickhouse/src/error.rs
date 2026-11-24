//! Why E6 stopped: the plan holds a value `ClickHouse` cannot express (`E6-01`), or the plan,
//! the model, the query or the security context breaks the contract of the stage that built
//! it.

use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_representations::origin::Origin;

/// How the input of E6 breaks the contract of the stages before it. Each one is a defect of
/// the stage that built the plan, not a query error, so it has no specification code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantViolation {
    /// A node the generator does not generate (VS-1).
    NodeOutsideSlice,
    /// An expression construct the 0.3.0 generator does not translate.
    ExpressionOutsideSlice,
    /// A `Scan` of a dataset the model does not have.
    UnknownDataset,
    /// A dataset `source` that names no relation of the catalog schema.
    UnknownRelation,
    /// A dataset `source` that is neither a query nor a qualified name (E6-4.3).
    MalformedSource,
    /// A `Scan` column that names no field of its dataset.
    UnknownField,
    /// A field without an expression tree.
    FieldWithoutExpression,
    /// A qualified reference: plan expressions name input columns only.
    QualifiedReference,
    /// A reference to a column the input node does not produce.
    UnknownColumn,
    /// An operand whose type the generator needs and cannot find (E6-7.6).
    UntypedOperand,
    /// A column or value of a type without a `ClickHouse` name (E6-7.2).
    UnsupportedType,
    /// `USER_ATTRIBUTE('name')` of an attribute the model does not declare.
    UndeclaredAttribute,
    /// `USER_ATTRIBUTE('name')` of an attribute the security context does not provide
    /// (`ESEC-02` is E4's to report, E4-6.9).
    MissingAttribute,
    /// A list attribute outside `x [NOT] IN (USER_ATTRIBUTE('name'))` (SCY-4.1).
    MisplacedListAttribute,
    /// A function call whose arguments do not match its function (E0F-10), or whose literal
    /// argument is not a literal integer.
    MalformedFunctionCall,
    /// `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP` in a query without a reference
    /// time (`EQRY-03` is E1's to report, E0F-2.3).
    MissingReferenceTime,
    /// A float literal without a finite value.
    NonFiniteFloat,
    /// A `Join` without key pairs (E4-3.2).
    JoinWithoutKeys,
    /// A `Join` key pair whose two types have no common type (E0F-3.1, E6-4.13).
    IncompatibleJoinKeys,
    /// A column of the right side of a left `Join` that the plan marks as not nullable
    /// (PLN-1).
    NonNullableJoinedColumn,
    /// A `Join` column that both inputs produce.
    AmbiguousColumn,
    /// A `Deduplicate` without identity columns (E4-4.0f).
    EmptyIdentity,
    /// A `SemiJoin` without key equalities (E4-3.4).
    EmptyJoinCondition,
    /// A `SemiJoin` along a relationship the model does not have.
    UnknownRelationship,
    /// A `Stitch` column whose branch index names no branch of the node (E4-3.7).
    UnknownBranch,
    /// A `Snapshot` rule that ADD-5.1.5 forbids for its `over`: `first`, `last` or
    /// `nulls: as_missing` across a field that is not a time field.
    SnapshotOutsideRule,
    /// A final node whose columns do not match the output columns of the query (E1-5).
    ResultColumnMismatch,
    /// A recomposed component whose call is not the one its recomposition reads, or that has a
    /// filter (SUM-3.1).
    MalformedRecomposition,
}

/// The message of `E6-01` (E6-7.12, E6-8).
const INEXPRESSIBLE_TYPE: &str = "this value is a `time`, an `interval` or a `timestamp` without \
    time zone, which the `clickhouse` dialect cannot express";

/// The error of E6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerateError {
    /// `E6-01`: the plan reads or returns a value whose logical type `ClickHouse` cannot
    /// express (E6-7.12), located at the origin of the first field or metric of that type.
    InexpressibleType {
        /// The origin of the field or metric.
        origin: Origin,
    },
    /// The input breaks the contract of the stages before E6. This is a defect of the
    /// compiler, not a query error, so it has no specification code.
    Invariant {
        /// What the input breaks.
        violation: InvariantViolation,
        /// The origin of the plan node where it is.
        origin: Origin,
    },
}

impl GenerateError {
    /// The error `violation` in the node that comes from `origin`.
    pub(crate) fn new(violation: InvariantViolation, origin: &Origin) -> Self {
        Self::Invariant {
            violation,
            origin: origin.clone(),
        }
    }

    /// What the input breaks, for an internal error; `None` for `E6-01`.
    #[must_use]
    pub fn violation(&self) -> Option<InvariantViolation> {
        match self {
            Self::InexpressibleType { .. } => None,
            Self::Invariant { violation, .. } => Some(*violation),
        }
    }

    /// The origin of the element where the error is.
    #[must_use]
    pub fn origin(&self) -> &Origin {
        match self {
            Self::InexpressibleType { origin } | Self::Invariant { origin, .. } => origin,
        }
    }

    /// The diagnostic of the compile error document (PLN-5): `E6-01` at its field or metric,
    /// in the query or the model; `None` for an internal error.
    #[must_use]
    pub fn to_diagnostic(&self) -> Option<Diagnostic> {
        let Self::InexpressibleType { origin } = self else {
            return None;
        };
        let location = match origin {
            Origin::Query(location) => {
                let position = location.position();
                DiagnosticLocation {
                    document: InputDocument::Query,
                    pointer: location.pointer().to_owned(),
                    line: position.and_then(|position| u64::try_from(position.line()).ok()),
                    column: position.and_then(|position| u64::try_from(position.column()).ok()),
                }
            }
            Origin::Model(location) => DiagnosticLocation {
                document: InputDocument::Model,
                pointer: location.pointer.clone(),
                line: u64::try_from(location.line).ok(),
                column: u64::try_from(location.column).ok(),
            },
        };
        Some(Diagnostic {
            code: "E6-01".to_owned(),
            message: INEXPRESSIBLE_TYPE.to_owned(),
            location: Some(location),
            alternatives: None,
        })
    }
}
