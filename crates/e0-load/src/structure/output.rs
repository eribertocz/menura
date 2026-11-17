//! The result of structure validation, as data types only.

use semantic_compiler_contracts::types::audit_record::OssieFormat;

use super::diagnostic::StructureDiagnostic;
use crate::pointer::JsonPointer;
use crate::read::Node;

/// A document whose structure is valid: the semantic model to compile, the compiled variant and
/// declared datatype of each of its expressions, its `nonportable_extensions` entries and the
/// load warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct Structure<'document> {
    /// The format the document was read in (load option `ossie_format`, MDL-1.1).
    pub format: OssieFormat,
    /// The semantic model to compile (E0B-3.2); the others are ignored.
    pub semantic_model: SelectedModel<'document>,
    /// One entry per field and per metric of the selected model, in document order.
    pub expressions: Vec<ElementExpression<'document>>,
    /// Every `nonportable_extensions` entry of the selected model, in document order (MDL-2.4).
    /// Always empty in the release format, whose schema does not allow the member; its
    /// `custom_extensions` entries stay in the data tree.
    pub nonportable_extensions: Vec<NonportableExtension<'document>>,
    /// `W0B-01`, `W0B-02` and `W0B-03` warnings, in document order then by code (E0-4.3).
    pub warnings: Vec<StructureDiagnostic>,
}

/// The semantic model selected by the load option `model_name` (E0B-3.2, DEV-7).
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedModel<'document> {
    /// Its index in `semantic_model`; always 0 in the development format, whose flat root is the
    /// one semantic model (section 4.5).
    pub index: usize,
    /// Its JSON Pointer, such as `/semantic_model/0`; the root (an empty pointer) in the development format.
    pub pointer: JsonPointer,
    /// Its node in the data tree.
    pub node: &'document Node,
}

/// What kind of element an expression belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementKind {
    /// A field of a dataset.
    Field,
    /// A metric of the semantic model.
    Metric,
    /// A dataset-scoped metric of the development format (section 4.5).
    DatasetMetric,
}

/// The expression of one field or metric (E0B-3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementExpression<'document> {
    /// Whether the element is a field or a metric.
    pub kind: ElementKind,
    /// The JSON Pointer of the element, such as `/semantic_model/0/datasets/2/fields/1`.
    pub pointer: JsonPointer,
    /// The variant that is compiled, or why there is none.
    pub variant: CompiledVariant<'document>,
    /// The development-line `datatype` of the element (OSS-5.2).
    pub datatype: DeclaredDatatype<'document>,
}

impl ElementExpression<'_> {
    /// The query error of an element without a single `ANSI_SQL` variant (E0-4 table).
    pub const NO_SINGLE_ANSI_SQL_QUERY_ERROR: &'static str = "E0B-08";
    /// The query error of an element with an unknown `datatype` (E0-4 table).
    pub const UNKNOWN_DATATYPE_QUERY_ERROR: &'static str = "E0B-09";

    /// Whether the element can be compiled.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        self.query_error().is_none()
    }

    /// The error code of a query that names the element, when it is unusable (E0-4.1). When both
    /// causes apply, `E0B-08` comes first, in the order of E2 step 12a.
    #[must_use]
    pub fn query_error(&self) -> Option<&'static str> {
        if self.variant == CompiledVariant::Unusable {
            Some(Self::NO_SINGLE_ANSI_SQL_QUERY_ERROR)
        } else if matches!(self.datatype, DeclaredDatatype::Unknown(_)) {
            Some(Self::UNKNOWN_DATATYPE_QUERY_ERROR)
        } else {
            None
        }
    }
}

/// The variant of an expression that is compiled (E0B-3.3, section 4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledVariant<'document> {
    /// The text of the single `ANSI_SQL` variant.
    AnsiSql(&'document str),
    /// The text of a development-line `sql` expression, interpreted as `OSSIE_SQL_2026`.
    OssieSql2026(&'document str),
    /// No `sql` and no single `ANSI_SQL` variant: the element is unusable (`W0B-02`, OSS-2.2),
    /// and a query that names it fails with [`ElementExpression::NO_SINGLE_ANSI_SQL_QUERY_ERROR`].
    Unusable,
}

/// The development-line `datatype` of a field or metric (OSS-5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredDatatype<'document> {
    /// The element has no `datatype`; it is always so in the release format.
    NotDeclared,
    /// The logical kind the `datatype` maps to.
    Mapped(LogicalKind),
    /// A `datatype` without a logical kind: the element is unusable (`W0B-03`), and a query
    /// that names it fails with [`ElementExpression::UNKNOWN_DATATYPE_QUERY_ERROR`].
    Unknown(&'document str),
}

/// The logical kind of a development-line `datatype` (OSS-5.2). Widths, precisions, scales and
/// timestamp precisions are not part of it: any of them matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalKind {
    /// `String`.
    Text,
    /// `Integer`, of any width.
    Integer,
    /// `Decimal`, of any precision and scale.
    Decimal,
    /// `Float`.
    Float,
    /// `Boolean`.
    Boolean,
    /// `Date`.
    Date,
    /// `DateTime` (without time zone) and `DateTimeTz` (with time zone).
    Timestamp {
        /// Whether it is an instant (`DateTimeTz`) rather than a wall-clock value (`DateTime`).
        with_time_zone: bool,
    },
    /// `Time`.
    Time,
}

/// The kind of object that carries a `nonportable_extensions` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtensionOwner {
    /// The semantic model.
    Model,
    /// A dataset.
    Dataset,
    /// A field of a dataset.
    Field,
    /// A dataset-scoped metric.
    DatasetMetric,
    /// A relationship.
    Relationship,
    /// A metric of the semantic model.
    Metric,
}

/// One development-line `nonportable_extensions` entry, whose `data` is an object (MDL-2.4).
/// Which entries are the compiler's own is decided by the common extension rules (E0.c).
#[derive(Debug, Clone, PartialEq)]
pub struct NonportableExtension<'document> {
    /// The kind of object that carries the entry.
    pub owner: ExtensionOwner,
    /// The JSON Pointer of that object, such as `/datasets/0`.
    pub owner_pointer: JsonPointer,
    /// The JSON Pointer of the entry, such as `/datasets/0/nonportable_extensions/0`.
    pub pointer: JsonPointer,
    /// Its `vendor_name`.
    pub vendor_name: &'document str,
    /// Its `data` object, as read.
    pub data: &'document Node,
}
