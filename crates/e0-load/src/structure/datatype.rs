//! The logical kind of a development-line `datatype` (OSS-5.2).

use super::diagnostic::{StructureCode, StructureDiagnostic};
use super::output::{DeclaredDatatype, LogicalKind};
use crate::nodes::member;
use crate::pointer::JsonPointer;
use crate::read::{Node, Value};

/// The `datatype` of `element`, located at `pointer`, with the `W0B-03` warning when it has no
/// logical kind.
///
/// The schema decides first (OSS-5.2): a value outside its `datatype` enumeration is already
/// `E0B-05`, so a value without a logical kind is one the schema accepts, today `Opaque`.
pub(crate) fn declared_datatype<'document>(
    element: &'document Node,
    pointer: &JsonPointer,
) -> (DeclaredDatatype<'document>, Option<StructureDiagnostic>) {
    let Some(node) = member(element, "datatype") else {
        return (DeclaredDatatype::NotDeclared, None);
    };
    let Value::String(datatype) = &node.value else {
        return (DeclaredDatatype::NotDeclared, None);
    };
    match logical_kind(datatype) {
        Some(kind) => (DeclaredDatatype::Mapped(kind), None),
        None => (
            DeclaredDatatype::Unknown(datatype),
            Some(StructureDiagnostic {
                code: StructureCode::UnknownDatatype,
                pointer: pointer.member("datatype"),
                location: node.location,
                message: format!(
                    "the datatype `{datatype}` has no logical kind; the element is unusable"
                ),
            }),
        ),
    }
}

/// The table of OSS-5.2.
fn logical_kind(datatype: &str) -> Option<LogicalKind> {
    match datatype {
        "String" => Some(LogicalKind::Text),
        "Integer" => Some(LogicalKind::Integer),
        "Decimal" => Some(LogicalKind::Decimal),
        "Float" => Some(LogicalKind::Float),
        "Boolean" => Some(LogicalKind::Boolean),
        "Date" => Some(LogicalKind::Date),
        "DateTime" => Some(LogicalKind::Timestamp {
            with_time_zone: false,
        }),
        "DateTimeTz" => Some(LogicalKind::Timestamp {
            with_time_zone: true,
        }),
        "Time" => Some(LogicalKind::Time),
        _ => None,
    }
}
