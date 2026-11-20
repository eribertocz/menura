//! `E0F-05`: the two columns of each relationship key pair have the same logical kind
//! (E0F-4.1, E0E-3.7).

use std::collections::HashMap;
use std::mem::discriminant;

use menura_contracts::types::common::LogicalType;

use super::diagnostic::{TypingCode, TypingDiagnostic};
use super::elements::Finding;
use super::output::ElementKind;
use super::scope::Scope;
use super::value_type::{ExpressionType, Typed, describe};
use crate::outline::{DatasetOutline, DeclaredName, RelationshipOutline};
use crate::pointer::JsonPointer;

/// One finding per relationship key pair whose types are incompatible, reading the types of
/// `fields`. A pair with a side that does not resolve, or that resolves to a field without a
/// type, is not checked: E0.d or the field's own check reports it.
pub(crate) fn key_findings(
    scope: &Scope<'_, '_>,
    fields: &HashMap<&JsonPointer, Typed>,
) -> Vec<Finding> {
    scope
        .model
        .relationships
        .iter()
        .flat_map(|relationship| relationship_findings(scope, fields, relationship))
        .collect()
}

fn relationship_findings(
    scope: &Scope<'_, '_>,
    fields: &HashMap<&JsonPointer, Typed>,
    relationship: &RelationshipOutline<'_>,
) -> Vec<Finding> {
    let end =
        |name: &Option<DeclaredName<'_>>| name.as_ref().and_then(|name| scope.dataset(&name.name));
    let (Some(from), Some(to)) = (end(&relationship.from), end(&relationship.to)) else {
        return Vec::new();
    };
    relationship
        .from_columns
        .iter()
        .zip(&relationship.to_columns)
        .filter_map(|(from_column, to_column)| {
            let from_type = key_type(scope, fields, from, from_column)?;
            let to_type = key_type(scope, fields, to, to_column)?;
            (!same_kind(&from_type, &to_type)).then(|| Finding {
                diagnostic: TypingDiagnostic {
                    code: TypingCode::IncompatibleKeyPair,
                    pointer: from_column.written.pointer.clone(),
                    location: from_column.written.location,
                    message: format!(
                        "the key `{}` ({}) cannot be paired with `{}` ({})",
                        from_column.written.text,
                        describe(&from_type),
                        to_column.written.text,
                        describe(&to_type)
                    ),
                },
                element: (ElementKind::Relationship, relationship.pointer.clone()),
            })
        })
        .collect()
}

/// The type of the key `key` of `dataset` (E0D-3.1): the field of that name, otherwise the
/// source column of that name.
fn key_type(
    scope: &Scope<'_, '_>,
    fields: &HashMap<&JsonPointer, Typed>,
    dataset: &DatasetOutline<'_>,
    key: &DeclaredName<'_>,
) -> Option<ExpressionType> {
    if let Some(field) = dataset.field(&key.name) {
        return fields
            .get(&field.pointer)
            .map(|typed| typed.value_type.clone());
    }
    let column = scope.relation(dataset)?.find(&key.name, key.delimited)?;
    Some(ExpressionType::Value(column.logical_type.clone()))
}

/// Whether two key types pair (E0F-4.1): the same logical kind; integers may differ in width,
/// and decimals must have the same scale. The literal `NULL` pairs with every type.
pub(super) fn same_kind(left: &ExpressionType, right: &ExpressionType) -> bool {
    match (left, right) {
        (ExpressionType::Value(left), ExpressionType::Value(right)) => match (left, right) {
            (
                LogicalType::Decimal { scale, .. },
                LogicalType::Decimal {
                    scale: other_scale, ..
                },
            ) => scale == other_scale,
            _ => discriminant(left) == discriminant(right),
        },
        _ => true,
    }
}
