//! The diagnostics of the common extension rules (E0C-3), as data types only.

use std::fmt;

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The code of an extension diagnostic: the common rules (E0C-3) and the codes each extension
/// chapter defines (E0C-2.5).
///
/// `E0C-03`, a wrong value type of an extension key, is raised by the validation of each
/// extension, not by the common rules. The order of the variants is the order of the codes, so
/// diagnostics of one node sort by code (E0-4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExtensionCode {
    /// `E0C-01`: `data.version` is missing or not supported.
    Version,
    /// `E0C-02`: a key of `data` that no extension defines.
    UnknownKey,
    /// `E0C-03`: a wrong value type, or a value outside its set, of an extension key of ours.
    WrongValue,
    /// `E0C-04`: more than one entry of ours on the same object.
    SeveralEntries,
    /// `E0C-05`: an extension key on an object type it is not valid on.
    WrongObject,
    /// `E0C-06`: a member name repeated in an object inside `data` of an entry of ours.
    RepeatedMember,
    /// `E0C-10`: a `non_additive` declaration has another key (ADD-5.1.1).
    NonAdditiveKey,
    /// `E0C-11`: an additivity declaration lacks `type`, or a `semi_additive` one lacks a
    /// required key (ADD-5.1.1).
    MissingAdditivityKey,
    /// `E0C-12`: an additivity option that does not match the kind of `over` (ADD-5.1.5).
    AdditivityTimeKind,
    /// `E0C-13`: `max_carry_forward` or `on_stale` without `missing: carry_forward`, or the
    /// other way round, or a `max_carry_forward` not greater than 0 (ADD-5.1.2, ADD-5.1.4).
    CarryForwardKeys,
    /// `E0C-14`: `selection` without `aggregation` `first` or `last`, or the other way round
    /// (ADD-5.1.4).
    SelectionPair,
    /// `E0C-15`: `over` is part of `entity` (ADD-5.1.3).
    OverInEntity,
    /// `E0C-16`: a field of `over` or `entity` that does not exist or is not reachable from
    /// the metric's home dataset (ADD-5.1.6).
    UnreachableAdditivityField,
    /// `E0C-17`: the population of an additivity declaration does not exist or does not match
    /// it (ADD-5.1.7).
    AdditivityPopulation,
    /// `E0C-20`: a population name repeated in the model (POP-4.3).
    DuplicatePopulation,
    /// `E0C-21`: a population lacks a key of its `mode`, or has a key of the other mode
    /// (POP-4.1).
    PopulationModeKeys,
    /// `E0C-22`: a population names a field its dataset does not have (POP-4.3).
    MissingPopulationField,
    /// `E0C-23`: a population `required_when` that is not a valid boolean expression over its
    /// dataset (POP-4.4).
    InvalidPopulationCondition,
    /// `E0C-24`: a population `valid_from`, `valid_to` or `period` field that is not of kind
    /// `date` or `timestamp` (POP-4.5).
    PopulationTimeKind,
    /// `E0C-30`: an aggregate function in a derived formula (DER-5.1.3).
    AggregateInFormula,
    /// `E0C-31`: a field reference in a derived formula (DER-5.1.3).
    FieldInFormula,
    /// `E0C-32`: `AGG(…)` names a metric that does not exist (DER-5.1.2).
    UnknownInput,
    /// `E0C-33`: a derived formula that refers to no metric (DER-5.1.2).
    NoInput,
    /// `E0C-34`: a dependency cycle between metrics (DER-5.1.4).
    DependencyCycle,
    /// `E0C-35`: a derived formula that divides without `on_division_by_zero` (DER-5.1.3).
    MissingDivisionRule,
    /// `E0C-36`: a derived or context metric with an `additivity` extension (DER-5.1.1,
    /// CTX-5.1.1).
    DerivedAdditivity,
    /// `E0C-37`: `AGG(…)` in a metric marked neither `derived` nor `context` (DER-5.1.2). The
    /// metric is unusable, reported as `W0G-01` (E0-4 table).
    AggregateOutsideDerived,
    /// `E0C-41`: `access` without `mode`, or with a `mode` outside SCY-3 (SCY-5.1.3).
    AccessMode,
    /// `E0C-42`: `mode: roles` with a missing or empty role list (SCY-5.1.3).
    AccessRoles,
    /// `E0C-43`: `roles` with a mode other than `roles`, or `fields` with `mode: internal`
    /// (SCY-5.1.3).
    AccessKeys,
    /// `E0C-44`: `mode: internal` on a dataset that is not a summary (SCY-5).
    InternalAccess,
    /// `E0C-45`: a row-policy name repeated in the model (SCY-5.1.5).
    DuplicatePolicy,
    /// `E0C-46`: a row-policy condition that is not a valid boolean expression over its
    /// dataset, or that reads an undeclared attribute (SCY-5.1.6).
    InvalidPolicyCondition,
    /// `E0C-47`: an undeclared role in `roles`, `exempt_roles` or `fields` (SCY-5.1.3,
    /// SCY-5.1.5).
    UndeclaredRole,
    /// `E0C-48`: a list attribute used outside `IN` / `NOT IN` (SCY-4.1, SCY-5.1.6).
    MisplacedListAttribute,
    /// `E0C-49`: `access.fields` names a missing field, or a role without access to the
    /// dataset (SCY-5.1.4).
    AccessFieldReference,
    /// `E0C-50`: a summary without `of`, or whose `of` names no dataset or a summary
    /// (SUM-6.1.1, SUM-6.1.2).
    SummaryBase,
    /// `E0C-51`: a summary `field` or `base` that does not exist, a summary field named by no
    /// dimension and no measure or by more than one, or a missing field reference in a
    /// `base_aggregate` (SUM-6.1.2).
    SummaryField,
    /// `E0C-52`: an invalid `base_aggregate` (SUM-2.1, SUM-6.1.3).
    InvalidBaseAggregate,
    /// `E0C-53`: a time dimension without `grain`, or a `grain` on a field that is not a time
    /// field (SUM-6.1.4).
    SummaryGrain,
    /// `E0C-54`: a `coverage_field` that is not a time dimension of the summary (SUM-6.1.2).
    CoverageField,
    /// `E0C-55`: a summary that declares `row_access`, or dataset metrics in the development
    /// format (SUM-6.1.6).
    SummaryPolicies,
    /// `E0C-56`: a summary without `access`, or with a valid mode other than `internal`
    /// (SUM-6.1.6).
    SummaryAccess,
    /// `E0C-57`: a summary field whose logical kind differs from its base (SUM-6.1.5).
    SummaryKind,
    /// `E0C-60`: a context `base` that names no model metric, an expression that is not
    /// `AGG(<base>)`, or a metric with both `context` and `derived` (CTX-5.1.3).
    ContextBase,
    /// `E0C-61`: a `time_dimension` that is not a time field reachable from the base
    /// (CTX-5.1.4).
    ContextTimeDimension,
    /// `E0C-62`: an `offset` that is not a positive integer (CTX-5.1.1).
    ContextOffset,
    /// `E0C-63`: a `remove` field that does not exist (CTX-5.1.3).
    RemovedFieldMissing,
    /// `E0C-64`: a context `base` that is itself a context metric (CTX-5.1.3).
    NestedContext,
    /// `W0C-01`: a `COMMON` entry whose `data` is not valid JSON; kept, not interpreted.
    NotJson,
    /// `W0C-40`: the model has no `security_context`; it declares no roles or attributes
    /// (SCY-2.1, SCY-5.1.7).
    NoSecurityContext,
    /// `W0C-41`: a dataset has no `access`; it is inaccessible (SCY-3.0, SCY-5.1.7).
    NoAccess,
}

impl ExtensionCode {
    /// The code as the specification writes it, such as `E0C-04`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Version => "E0C-01",
            Self::UnknownKey => "E0C-02",
            Self::WrongValue => "E0C-03",
            Self::SeveralEntries => "E0C-04",
            Self::WrongObject => "E0C-05",
            Self::RepeatedMember => "E0C-06",
            Self::NonAdditiveKey => "E0C-10",
            Self::MissingAdditivityKey => "E0C-11",
            Self::AdditivityTimeKind => "E0C-12",
            Self::CarryForwardKeys => "E0C-13",
            Self::SelectionPair => "E0C-14",
            Self::OverInEntity => "E0C-15",
            Self::UnreachableAdditivityField => "E0C-16",
            Self::AdditivityPopulation => "E0C-17",
            Self::DuplicatePopulation => "E0C-20",
            Self::PopulationModeKeys => "E0C-21",
            Self::MissingPopulationField => "E0C-22",
            Self::InvalidPopulationCondition => "E0C-23",
            Self::PopulationTimeKind => "E0C-24",
            Self::AggregateInFormula => "E0C-30",
            Self::FieldInFormula => "E0C-31",
            Self::UnknownInput => "E0C-32",
            Self::NoInput => "E0C-33",
            Self::DependencyCycle => "E0C-34",
            Self::MissingDivisionRule => "E0C-35",
            Self::DerivedAdditivity => "E0C-36",
            Self::AggregateOutsideDerived => "E0C-37",
            Self::AccessMode => "E0C-41",
            Self::AccessRoles => "E0C-42",
            Self::AccessKeys => "E0C-43",
            Self::InternalAccess => "E0C-44",
            Self::DuplicatePolicy => "E0C-45",
            Self::InvalidPolicyCondition => "E0C-46",
            Self::UndeclaredRole => "E0C-47",
            Self::MisplacedListAttribute => "E0C-48",
            Self::AccessFieldReference => "E0C-49",
            Self::SummaryBase => "E0C-50",
            Self::SummaryField => "E0C-51",
            Self::InvalidBaseAggregate => "E0C-52",
            Self::SummaryGrain => "E0C-53",
            Self::CoverageField => "E0C-54",
            Self::SummaryPolicies => "E0C-55",
            Self::SummaryAccess => "E0C-56",
            Self::SummaryKind => "E0C-57",
            Self::ContextBase => "E0C-60",
            Self::ContextTimeDimension => "E0C-61",
            Self::ContextOffset => "E0C-62",
            Self::RemovedFieldMissing => "E0C-63",
            Self::NestedContext => "E0C-64",
            Self::NotJson => "W0C-01",
            Self::NoSecurityContext => "W0C-40",
            Self::NoAccess => "W0C-41",
        }
    }

    /// The load warning that `E0C-37` is reported as: the metric is unusable (E0-4 table).
    pub const UNUSABLE_WARNING: &'static str = "W0G-01";

    /// Whether the code is a load warning: `W0C-01`, `W0C-40`, `W0C-41`, and `E0C-37`, which
    /// makes its metric unusable. Every other code rejects the model (E0-4.1).
    #[must_use]
    pub fn is_warning(self) -> bool {
        matches!(
            self,
            Self::NotJson
                | Self::NoSecurityContext
                | Self::NoAccess
                | Self::AggregateOutsideDerived
        )
    }

    /// The code the load reports: [`Self::UNUSABLE_WARNING`] for `E0C-37`, whose own code is
    /// then the query error of the metric (E0-4.1), and the code itself otherwise.
    #[must_use]
    pub fn load_code(self) -> &'static str {
        match self {
            Self::AggregateOutsideDerived => Self::UNUSABLE_WARNING,
            code => code.as_str(),
        }
    }
}

impl fmt::Display for ExtensionCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One common extension diagnostic: its code, the node it is about and a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDiagnostic {
    /// The specification code.
    pub code: ExtensionCode,
    /// The JSON Pointer of the node (E0B-3.6): the `data` member of the entry, or the entry
    /// itself for `E0C-04`. A problem inside `data` cannot be located deeper, because `data` is a
    /// JSON text.
    pub pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub location: Location,
    /// What the problem is, in English, for diagnostics.
    pub message: String,
}

impl fmt::Display for ExtensionDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let load_code = self.code.load_code();
        if load_code == self.code.as_str() {
            write!(formatter, "{load_code}")?;
        } else {
            write!(formatter, "{load_code} ({})", self.code)?;
        }
        write!(
            formatter,
            " at `{}` (line {}, column {}): {}",
            self.pointer, self.location.line, self.location.column, self.message
        )
    }
}

impl std::error::Error for ExtensionDiagnostic {}
