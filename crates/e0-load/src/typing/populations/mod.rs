//! `E0C-23`, `E0C-24`, `E0C-66` and `E0C-72`: the condition, the time fields, the `open_end`
//! and the kinds of an exclusive range of every population (POP-4.4, POP-4.5, VAL-1.5), typed
//! against the field types E0.f found.
//!
//! `required_when` is a row-level expression of the declaring dataset: a bare name is a field
//! of that dataset. A text that does not parse, a construct outside the subset, a qualified
//! reference, an aggregate, `AGG(…)`, `USER_ATTRIBUTE(…)`, or a type other than boolean is
//! `E0C-23`. The structural conditions are checked whatever the types of the fields read; the
//! type is checked only when every field read has a type, and `E0C-24` only on typed fields: a
//! field without a type already has its own diagnostic (POP-4.5). An `open_end` is read as a
//! value of the type of `valid_to` only when `valid_to` has a `date` or `timestamp` type: any
//! other type already has its `E0C-24`. Under `valid_to_bound: exclusive`, `valid_from` and
//! `valid_to` must both be `date`, or both `timestamp` with the same `with_time_zone`; that is
//! checked only when both have a type. A missing field is `E0C-22`, reported by E0.d.

pub(super) mod structure;

use std::collections::HashMap;

use menura_contracts::types::common::LogicalType;
use menura_model::expression::Reference;
use menura_model::internal_model::ValidToBound;

use super::output::{ElementKind, Typing};
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::populations::{PopulationMode, parse_open_end};
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Population, per_entry};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::structure::Structure;
use menura_expressions::typing::{Aggregates, TypingScope, type_expression};
use menura_expressions::typing::{Outcome, Stop};
use menura_expressions::typing::{Typed, describe};
use structure::{bare_references, structural_problem};

/// The field types and datasets the checks of populations read.
struct Fields<'model, 'document> {
    datasets: HashMap<&'model JsonPointer, &'model DatasetOutline<'document>>,
    types: HashMap<&'model JsonPointer, &'model Typed>,
}

impl Fields<'_, '_> {
    /// The type of the field `name` of the dataset at `dataset`, if it exists and E0.f typed
    /// it.
    fn type_of(&self, dataset: &JsonPointer, name: &Name) -> Option<&Typed> {
        let field = self.datasets.get(dataset)?.field(name)?;
        self.types.get(&field.pointer).copied()
    }
}

/// Types the condition and the time fields of every population of `extensions`, in the
/// semantic model that `structure` selected, with the field types of `typing`.
///
/// # Errors
///
/// Returns at most one `E0C-23`, one `E0C-24`, one `E0C-66` and one `E0C-72` diagnostic per
/// entry, at the entry's `data` member (E0C-2.6), in document order then by code. Every one
/// of these codes rejects the model (E0-4).
pub fn type_populations(
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
    let populations = &extensions.populations;
    let mut errors = per_entry(
        populations,
        ExtensionCode::InvalidPopulationCondition,
        "a population required_when is not a valid boolean expression over its dataset",
        |population| condition_problem(&fields, population).into_iter().collect(),
    );
    errors.extend(per_entry(
        populations,
        ExtensionCode::PopulationTimeKind,
        "a population time field is not of kind date or timestamp",
        |population| time_problems(&fields, population),
    ));
    errors.extend(range_shape_problems(&fields, populations));
    let order: HashMap<&JsonPointer, usize> = populations
        .iter()
        .enumerate()
        .rev()
        .map(|(index, population)| (&population.data, index))
        .collect();
    errors.sort_by_key(|diagnostic| (order.get(&diagnostic.pointer).copied(), diagnostic.code));
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The `E0C-66` and `E0C-72` diagnostics of `populations`: their range shape (VAL-1.5).
fn range_shape_problems(
    fields: &Fields<'_, '_>,
    populations: &[Population],
) -> Vec<ExtensionDiagnostic> {
    let mut errors = per_entry(
        populations,
        ExtensionCode::InvalidOpenEnd,
        "a population open_end is not a value of the type of its valid_to",
        |population| open_end_problem(fields, population).into_iter().collect(),
    );
    errors.extend(per_entry(
        populations,
        ExtensionCode::ExclusiveRangeKinds,
        "an exclusive population range has valid_from and valid_to of different kinds",
        |population| {
            exclusive_kinds_problem(fields, population)
                .into_iter()
                .collect()
        },
    ));
    errors
}

/// Why the `required_when` of `population` is not a valid boolean expression (POP-4.4).
fn condition_problem(fields: &Fields<'_, '_>, population: &Population) -> Option<String> {
    let text = population.required_when.as_ref()?;
    let described = |problem: String| {
        format!(
            "the required_when of the population `{}`: {problem}",
            population.name
        )
    };
    let tree = match parse_element_expression(text, ExpressionPlace::Extension) {
        Ok(tree) => tree,
        Err(diagnostic) => return Some(described(diagnostic.error.to_string())),
    };
    if let Some(problem) = structural_problem(&tree) {
        return Some(described(problem));
    }
    let all_typed = bare_references(&tree).into_iter().all(|reference| {
        fields
            .type_of(&population.dataset, &Name::of(&reference.name))
            .is_some()
    });
    if !all_typed {
        return None;
    }
    let resolve = |reference: &Reference| field_type(fields, population, reference);
    let condition_typer = TypingScope::new(&resolve, Aggregates::NotAllowed);
    let problem = match type_expression(&tree, &condition_typer) {
        Ok(found) if found.logical() == Some(&LogicalType::Boolean) => return None,
        Ok(found) => format!("it is {}, not boolean", describe(&found.value_type)),
        Err(Stop::Problem { message, .. }) => message,
        Err(Stop::ReportedElsewhere) => return None,
    };
    Some(described(problem))
}

/// The type of a bare reference of the condition of `population`: a field of its dataset
/// (POP-4.4). The structural check has already rejected qualified references.
fn field_type(
    fields: &Fields<'_, '_>,
    population: &Population,
    reference: &Reference,
) -> Outcome<Typed> {
    fields
        .type_of(&population.dataset, &Name::of(&reference.name))
        .cloned()
        .ok_or(Stop::ReportedElsewhere)
}

/// The time fields of `population` that E0.f typed with a kind other than `date` or
/// `timestamp` (POP-4.5).
fn time_problems(fields: &Fields<'_, '_>, population: &Population) -> Vec<String> {
    population
        .time_fields()
        .into_iter()
        .filter_map(|field| {
            let typed = fields.type_of(&population.dataset, &Name::declared(field))?;
            let temporal = matches!(
                typed.logical(),
                Some(LogicalType::Date | LogicalType::Timestamp { .. })
            );
            (!temporal).then(|| {
                format!(
                    "`{field}` of the population `{}` is {}",
                    population.name,
                    describe(&typed.value_type)
                )
            })
        })
        .collect()
}

/// Why the `open_end` of `population` is not a value of the type of its `valid_to` in its
/// CON-4 form (VAL-1.5): checked only when `valid_to` has a `date` or `timestamp` type.
fn open_end_problem(fields: &Fields<'_, '_>, population: &Population) -> Option<String> {
    let PopulationMode::Validity {
        valid_to,
        open_end: Some(open_end),
        ..
    } = &population.mode
    else {
        return None;
    };
    let typed = fields.type_of(&population.dataset, &Name::declared(valid_to))?;
    let logical_type = typed
        .logical()
        .filter(|logical| matches!(logical, LogicalType::Date | LogicalType::Timestamp { .. }))?;
    parse_open_end(open_end, logical_type).is_none().then(|| {
        format!(
            "the open_end `{open_end}` of the population `{}` is not a {} in its CON-4 form",
            population.name,
            describe(&typed.value_type)
        )
    })
}

/// Why `valid_from` and `valid_to` of an exclusive `population` cannot be compared without a
/// time zone (VAL-1.5): they are not both `date`, or not both `timestamp` with the same
/// `with_time_zone`. Checked only when both have a type.
fn exclusive_kinds_problem(fields: &Fields<'_, '_>, population: &Population) -> Option<String> {
    let PopulationMode::Validity {
        valid_from,
        valid_to,
        valid_to_bound: ValidToBound::Exclusive,
        ..
    } = &population.mode
    else {
        return None;
    };
    let from = fields.type_of(&population.dataset, &Name::declared(valid_from))?;
    let to = fields.type_of(&population.dataset, &Name::declared(valid_to))?;
    let same_kind = match (from.logical(), to.logical()) {
        (Some(LogicalType::Date), Some(LogicalType::Date)) => true,
        (
            Some(LogicalType::Timestamp {
                with_time_zone: from_zone,
                ..
            }),
            Some(LogicalType::Timestamp {
                with_time_zone: to_zone,
                ..
            }),
        ) => from_zone == to_zone,
        _ => false,
    };
    (!same_kind).then(|| {
        format!(
            "the exclusive population `{}` has `{valid_from}` of type {} and `{valid_to}` of type {}",
            population.name,
            describe(&from.value_type),
            describe(&to.value_type)
        )
    })
}
