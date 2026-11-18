//! `E0C-23` and `E0C-24`: the condition and the time fields of every population (POP-4.4,
//! POP-4.5), typed against the field types E0.f found.
//!
//! `required_when` is a row-level expression of the declaring dataset: a bare name is a field
//! of that dataset. A text that does not parse, a construct outside the subset, a qualified
//! reference, an aggregate, `AGG(metric)`, `USER_ATTRIBUTE`, or a type other than boolean is
//! `E0C-23`. A field without a type already has its own diagnostic, so neither code is reported
//! for it (POP-4.5); a missing field is `E0C-22`, reported by E0.d.

use std::collections::HashMap;

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::Reference;

use super::output::{ElementKind, Typing};
use super::stop::{Outcome, Stop, not_allowed};
use super::typer::{Aggregates, ExpressionTyper};
use super::value_type::{Typed, describe};
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Population, per_entry};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::structure::Structure;

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
/// Returns at most one `E0C-23` and one `E0C-24` diagnostic per entry, at the entry's `data`
/// member (E0C-2.6), in document order then by code. Both codes reject the model (E0-4).
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
    let resolve = |reference: &Reference| field_type(fields, population, reference);
    let condition_typer = ExpressionTyper::new(&resolve, Aggregates::Condition);
    let problem = match condition_typer.type_of(&tree) {
        Ok(found) if found.logical() == Some(&LogicalType::Boolean) => return None,
        Ok(found) => format!("it is {}, not boolean", describe(&found.value_type)),
        Err(Stop::Problem { message, .. } | Stop::NotAllowed { message }) => message,
        Err(Stop::ReportedElsewhere) => return None,
    };
    Some(described(problem))
}

/// The type of a reference of the condition of `population`: a bare name is a field of its
/// dataset (POP-4.4). A missing field is `E0C-22`, and a field without a type has its own
/// diagnostic.
fn field_type(
    fields: &Fields<'_, '_>,
    population: &Population,
    reference: &Reference,
) -> Outcome<Typed> {
    if let Some(qualifier) = &reference.qualifier {
        return not_allowed(format!(
            "the qualified reference `{}`.`{}`; a field of the dataset is read by its bare name",
            qualifier.text(),
            reference.name.text()
        ));
    }
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
