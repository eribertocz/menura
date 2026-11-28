//! `E0C-70` and `E0C-71`: the time of every temporal relationship (VAL-2.5, VAL-2.6), against
//! the field types E0.f found.
//!
//! `from_time`, and the `valid_from` and `valid_to` of the relationship's population, are all
//! `date`, or all `timestamp` with the same `with_time_zone`; that is checked only when all
//! three have a type (POP-4.5). Timestamps of different precisions are allowed (VAL-2.9). A
//! `from_time` field whose expression reads `CURRENT_DATE`, `CURRENT_TIME` or
//! `CURRENT_TIMESTAMP` is `E0C-71`; a field expression reads source columns only, never other
//! fields (E0D-3), so its own tree is all it reads. A name that does not resolve is reported
//! by E0.d.

use std::collections::HashMap;

use menura_contracts::types::common::LogicalType;
use menura_model::expression::Expression;

use super::output::{ElementKind, Typing};
use crate::expression_tree::nodes;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::populations::PopulationMode;
use crate::extensions::temporal::TemporalDeclaration;
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions};
use crate::names::Name;
use crate::outline::{DatasetOutline, DeclaredName, ElementOutline, ModelOutline, outline};
use crate::pointer::JsonPointer;
use crate::structure::{CompiledVariant, Structure};
use menura_expressions::typing::{Typed, describe};

/// The time fields of one temporal relationship, each with its pointer when it resolves.
struct TimeFields<'model, 'document> {
    from_time: Option<&'model ElementOutline<'document>>,
    valid_from: Option<&'model ElementOutline<'document>>,
    valid_to: Option<&'model ElementOutline<'document>>,
}

/// Checks the time of every temporal relationship of `extensions`, in the semantic model that
/// `structure` selected, with the field types of `typing`.
///
/// # Errors
///
/// Returns at most one `E0C-70` and one `E0C-71` diagnostic per declaration, at the `data`
/// member of the relationship's entry (E0C-2.6), in document order then by code. Both reject
/// the model (E0-4).
pub fn type_temporal(
    structure: &Structure<'_>,
    typing: &Typing,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let types: HashMap<&JsonPointer, &Typed> = typing
        .elements
        .iter()
        .filter(|element| element.kind == ElementKind::Field)
        .map(|element| (&element.pointer, &element.typed))
        .collect();
    let mut errors = Vec::new();
    for declaration in &extensions.temporal {
        let fields = time_fields(&model, extensions, declaration);
        let mut problems = Vec::new();
        problems.extend(kinds_problem(&types, &fields, declaration));
        problems.extend(clock_problem(structure, &fields, declaration));
        errors.extend(
            problems
                .into_iter()
                .map(|(code, message)| ExtensionDiagnostic {
                    code,
                    pointer: declaration.data.clone(),
                    location: declaration.location,
                    message,
                }),
        );
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The `from_time` field of the relationship's `from` dataset and the `valid_from` and
/// `valid_to` fields of its population, each when it resolves.
fn time_fields<'model, 'document>(
    model: &'model ModelOutline<'document>,
    extensions: &Extensions,
    declaration: &TemporalDeclaration,
) -> TimeFields<'model, 'document> {
    let relationship = model
        .relationships
        .iter()
        .find(|relationship| relationship.pointer == declaration.relationship);
    let dataset = |end: Option<&DeclaredName<'_>>| -> Option<&'model DatasetOutline<'document>> {
        let end = end?;
        model.datasets.iter().find(|dataset| {
            dataset
                .name
                .as_ref()
                .is_some_and(|name| name.name == end.name)
        })
    };
    let from = relationship.and_then(|relationship| dataset(relationship.from.as_ref()));
    let to = relationship.and_then(|relationship| dataset(relationship.to.as_ref()));
    let wanted = Name::declared(&declaration.population);
    let range = to.and_then(|to| {
        extensions.populations.iter().find_map(|population| {
            let PopulationMode::Validity {
                valid_from,
                valid_to,
                ..
            } = &population.mode
            else {
                return None;
            };
            (population.dataset == to.pointer && Name::declared(&population.name) == wanted)
                .then_some((to, valid_from, valid_to))
        })
    });
    TimeFields {
        from_time: from.and_then(|from| from.field(&Name::declared(&declaration.from_time))),
        valid_from: range.and_then(|(to, valid_from, _)| to.field(&Name::declared(valid_from))),
        valid_to: range.and_then(|(to, _, valid_to)| to.field(&Name::declared(valid_to))),
    }
}

/// `E0C-70` when the three time fields are typed and are not all `date`, or not all
/// `timestamp` with the same `with_time_zone` (VAL-2.5).
fn kinds_problem(
    types: &HashMap<&JsonPointer, &Typed>,
    fields: &TimeFields<'_, '_>,
    declaration: &TemporalDeclaration,
) -> Option<(ExtensionCode, String)> {
    let type_of = |field: Option<&ElementOutline<'_>>| {
        field.and_then(|field| types.get(&field.pointer).copied())
    };
    let found = [
        type_of(fields.from_time)?,
        type_of(fields.valid_from)?,
        type_of(fields.valid_to)?,
    ];
    let kinds: Vec<Option<TimeKind>> = found.iter().map(|typed| time_kind(typed)).collect();
    let comparable = kinds
        .first()
        .is_some_and(|first| first.is_some() && kinds.iter().all(|kind| kind == first));
    let [from_time, valid_from, valid_to] = found.map(|typed| describe(&typed.value_type));
    (!comparable).then(|| {
        (
            ExtensionCode::TemporalKinds,
            format!(
                "the temporal relationship compares from_time `{}` of type {from_time} with valid_from of type {valid_from} and valid_to of type {valid_to}; all three must be date, or timestamp with the same with_time_zone",
                declaration.from_time
            ),
        )
    })
}

/// The kind a time field is compared as (VAL-2.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeKind {
    /// Days.
    Date,
    /// Instants, with time zone, or wall-clock values, without.
    Timestamp {
        /// Whether it is an instant.
        with_time_zone: bool,
    },
}

/// The time kind of `typed`; `None` for any kind other than `date` or `timestamp`.
fn time_kind(typed: &Typed) -> Option<TimeKind> {
    match typed.logical()? {
        LogicalType::Date => Some(TimeKind::Date),
        LogicalType::Timestamp { with_time_zone, .. } => Some(TimeKind::Timestamp {
            with_time_zone: *with_time_zone,
        }),
        _ => None,
    }
}

/// `E0C-71` when the `from_time` field reads the clock (VAL-2.6).
fn clock_problem(
    structure: &Structure<'_>,
    fields: &TimeFields<'_, '_>,
    declaration: &TemporalDeclaration,
) -> Option<(ExtensionCode, String)> {
    let field = fields.from_time?;
    let text = structure
        .expressions
        .iter()
        .find(|expression| expression.pointer == field.pointer)
        .and_then(|expression| match expression.variant {
            CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text) => Some(text),
            CompiledVariant::Unusable => None,
        })?;
    let tree = parse_element_expression(text, ExpressionPlace::Field).ok()?;
    let reads_clock = nodes(&tree).into_iter().any(|node| {
        matches!(
            node,
            Expression::CurrentDate | Expression::CurrentTime | Expression::CurrentTimestamp
        )
    });
    reads_clock.then(|| {
        (
            ExtensionCode::TemporalClock,
            format!(
                "the temporal from_time `{}` reads CURRENT_DATE, CURRENT_TIME or CURRENT_TIMESTAMP; the version a row joins must not depend on when the query runs",
                declaration.from_time
            ),
        )
    })
}
