//! Names as E2 compares them (E0D-1, E2-5.5) and as its messages write them.

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::{Identifier, Reference};
use menura_model::internal_model::NormalizedName;
use menura_representations::names::OutputName;
use menura_representations::query_tree::{QueryReference, ReferencePart};

/// The normalized names of the parts of `reference` (E0D-1.1, E0D-1.4), or `None` when a part
/// names nothing: an empty part or a malformed delimited part (E1-3.1).
pub(crate) fn reference_names(reference: &QueryReference) -> Option<Vec<NormalizedName>> {
    reference.parts().iter().map(part_name).collect()
}

/// The normalized name of one part, or `None` when it names nothing.
fn part_name(part: &ReferencePart) -> Option<NormalizedName> {
    match part {
        ReferencePart::Regular(text) if !text.is_empty() => {
            Some(NormalizedName::new(&normalize_unquoted_identifier(text)))
        }
        ReferencePart::Delimited(text) if !text.is_empty() => Some(NormalizedName::new(text)),
        ReferencePart::Regular(_) | ReferencePart::Delimited(_) | ReferencePart::Malformed(_) => {
            None
        }
    }
}

/// The normalized name of an identifier of an expression (E0D-1.1).
pub(crate) fn identifier_name(identifier: &Identifier) -> NormalizedName {
    if identifier.is_delimited() {
        NormalizedName::new(identifier.text())
    } else {
        NormalizedName::new(&normalize_unquoted_identifier(identifier.text()))
    }
}

/// The key two output names collide on: the upper-case mapping of E0D-1.1, whatever their
/// origin (E2-5.5).
pub(crate) fn output_key(name: &OutputName) -> String {
    normalize_unquoted_identifier(name.as_str())
}

/// `reference` as written, for a message.
pub(crate) fn written_reference(reference: &QueryReference) -> String {
    let parts: Vec<String> = reference
        .parts()
        .iter()
        .map(|part| match part {
            ReferencePart::Regular(text) | ReferencePart::Malformed(text) => text.clone(),
            ReferencePart::Delimited(text) => delimited(text),
        })
        .collect();
    parts.join(".")
}

/// The expression reference `reference` as written, for a message.
pub(crate) fn written_expression_reference(reference: &Reference) -> String {
    match &reference.qualifier {
        Some(qualifier) => format!(
            "{}.{}",
            written_identifier(qualifier),
            written_identifier(&reference.name)
        ),
        None => written_identifier(&reference.name),
    }
}

/// The identifier `identifier` as written, for a message.
pub(crate) fn written_identifier(identifier: &Identifier) -> String {
    if identifier.is_delimited() {
        delimited(identifier.text())
    } else {
        identifier.text().to_owned()
    }
}

/// `text` as a delimited identifier: between `"`, with `"` doubled.
fn delimited(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}
