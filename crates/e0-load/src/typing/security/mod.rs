//! `E0C-46` and `E0C-48`: the condition of every row policy, parsed and typed as a row-level
//! expression of its dataset (SCY-4.1, SCY-4.2, SCY-5.1.6).
//!
//! A bare name is a field of the dataset, and `USER_ATTRIBUTE('name')` has the type its
//! declaration in `security_context` gives. A text that does not parse, a construct outside
//! the subset, a qualified reference, a missing field, an aggregate, `AGG(…)`, an undeclared
//! attribute, or a type other than boolean is `E0C-46`; a list attribute anywhere but as the
//! single item of an `IN` / `NOT IN` list is `E0C-48`, as the `security` crate decides (S5).
//! The structural conditions are checked whatever the field types; the boolean type only when
//! every field read has a type, and the condition has no misplaced list attribute.

mod condition;

use std::collections::HashMap;

use super::output::{ElementKind, Typing};
use super::value_type::Typed;
use crate::extensions::security::RowPolicy;
use crate::extensions::{ExtensionDiagnostic, Extensions, report_at};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::structure::Structure;
use condition::condition_problems;

/// The datasets and field types the conditions of row policies read.
pub(super) struct Fields<'model, 'document> {
    datasets: HashMap<&'model JsonPointer, &'model DatasetOutline<'document>>,
    types: HashMap<&'model JsonPointer, &'model Typed>,
}

/// What the field `name` of the dataset at `dataset` is to a condition.
pub(super) enum FieldLookup<'typed> {
    /// The dataset has no such field.
    Missing,
    /// The field exists, but E0.f gave it no type.
    Untyped,
    /// The field and its type.
    Typed(&'typed Typed),
}

impl Fields<'_, '_> {
    /// What the field `name` of the dataset at `dataset` is.
    pub(super) fn lookup(&self, dataset: &JsonPointer, name: &Name) -> FieldLookup<'_> {
        let Some(field) = self
            .datasets
            .get(dataset)
            .and_then(|dataset| dataset.field(name))
        else {
            return FieldLookup::Missing;
        };
        match self.types.get(&field.pointer) {
            Some(typed) => FieldLookup::Typed(typed),
            None => FieldLookup::Untyped,
        }
    }
}

/// Parses and types the condition of every row policy of `extensions`, in the semantic model
/// that `structure` selected, with the field types of `typing`.
///
/// # Errors
///
/// Returns at most one `E0C-46` and one `E0C-48` diagnostic per entry, at the entry's `data`
/// member (E0C-2.6), in document order then by code. Both codes reject the model (E0-4).
pub fn type_row_policies(
    structure: &Structure<'_>,
    typing: &Typing,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let fields = Fields {
        datasets: model
            .datasets
            .iter()
            .map(|dataset| (&dataset.pointer, dataset))
            .collect(),
        types: typing
            .elements
            .iter()
            .filter(|element| element.kind == ElementKind::Field)
            .map(|element| (&element.pointer, &element.typed))
            .collect(),
    };
    let security = &extensions.security;
    let mut errors = Vec::new();
    let mut groups: Vec<(&RowPolicy, Vec<_>)> = Vec::new();
    for policy in &security.policies {
        let found = condition_problems(&fields, security.context.as_ref(), policy);
        match groups.last_mut() {
            Some((first, problems)) if first.data == policy.data => problems.extend(found),
            _ => groups.push((policy, found)),
        }
    }
    for (first, problems) in groups {
        errors.extend(report_at(&first.data, first.location, problems));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
