//! Why E6 stopped: the plan, the model, the query or the security context breaks the contract
//! of the stage that built it.

use menura_representations::origin::Origin;

/// How the input of E6 breaks the contract of the stages before it. Each one is a defect of
/// the stage that built the plan, not a query error, so it has no specification code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantViolation {
    /// A `Join` node, which the generator does not generate yet (VS-1).
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
    /// An operand whose type the generator needs and cannot find (E6-5.2).
    UntypedOperand,
    /// A column or value of a type without a `DuckDB` name (E6-5.6).
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
    /// A `Deduplicate` without identity columns (E4-4.0f).
    EmptyIdentity,
    /// A `SemiJoin` without steps (E4-3.4).
    EmptyPath,
    /// A `SemiJoin` step whose relationship the model does not have, or that does not lead
    /// from its `from` dataset to its `to` dataset.
    UnknownRelationship,
    /// A `Stitch` column whose branch index names no branch of the node (E4-3.7).
    UnknownBranch,
    /// A final node whose columns do not match the output columns of the query (E1-5).
    ResultColumnMismatch,
}

/// The error of E6: what is broken, and the origin of the plan node where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateError {
    /// What the input breaks.
    pub violation: InvariantViolation,
    /// The origin of the plan node where it is.
    pub origin: Origin,
}

impl GenerateError {
    /// The error `violation` in the node that comes from `origin`.
    pub(crate) fn new(violation: InvariantViolation, origin: &Origin) -> Self {
        Self {
            violation,
            origin: origin.clone(),
        }
    }
}
