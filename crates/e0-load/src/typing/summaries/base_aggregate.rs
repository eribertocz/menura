//! The shape and the result type of one `base_aggregate` (SUM-2.1, SUM-6.1.3).

use menura_model::expression::{AggregateCall, Expression, Reference, SimpleAggregate};

use super::Fields;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::summaries::SummaryMeasure;
use crate::names::Name;
use crate::outline::DatasetOutline;
use crate::typing::populations::structure::nodes;
use crate::typing::stop::{Outcome, Stop};
use crate::typing::typer::{Aggregates, ExpressionTyper};
use crate::typing::value_type::Typed;

/// What typing a `base_aggregate` found.
pub(super) enum MeasureType {
    /// Its result type (E0F-5).
    Typed(Typed),
    /// It is not valid (`E0C-52`), for the reason given.
    Invalid(String),
    /// It has a valid shape, and a field it reads has no type: its type is not checked.
    Unchecked,
}

/// The result type of the `base_aggregate` of `measure`, over `base`, the dataset named
/// `of`.
pub(super) fn measure_type(
    fields: &Fields<'_, '_>,
    base: &DatasetOutline<'_>,
    of: &str,
    measure: &SummaryMeasure,
) -> MeasureType {
    let tree = match parse_element_expression(&measure.base_aggregate, ExpressionPlace::Metric) {
        Ok(tree) => tree,
        Err(diagnostic) => return MeasureType::Invalid(diagnostic.error.to_string()),
    };
    if let Some(problem) = shape_problem(&tree, &Name::declared(of)) {
        return MeasureType::Invalid(problem);
    }
    let all_typed = read_fields(&tree)
        .into_iter()
        .all(|reference| fields.type_of(base, &Name::of(&reference.name)).is_some());
    if !all_typed {
        return MeasureType::Unchecked;
    }
    let resolve = |reference: &Reference| field_type(fields, base, reference);
    let aggregate_typer = ExpressionTyper::new(&resolve, Aggregates::Allowed);
    match aggregate_typer.type_of(&tree) {
        Ok(typed) if typed.logical().is_some() => MeasureType::Typed(typed),
        Ok(_) => MeasureType::Invalid("it has no type".to_owned()),
        Err(Stop::Problem { message, .. }) => MeasureType::Invalid(message),
        Err(Stop::ReportedElsewhere) => MeasureType::Unchecked,
    }
}

/// Why `tree` is not one `SUM`, `COUNT(x)`, `MIN` or `MAX` call without `DISTINCT`, whose
/// argument and filter are row-level expressions reading only the dataset `of`.
fn shape_problem(tree: &Expression, of: &Name) -> Option<String> {
    let Expression::Aggregate {
        call: AggregateCall::Simple { function, argument },
        filter,
    } = tree
    else {
        return Some(
            "it is not a single call of SUM, COUNT(x), MIN or MAX without DISTINCT".to_owned(),
        );
    };
    if !matches!(
        function,
        SimpleAggregate::Sum
            | SimpleAggregate::Count
            | SimpleAggregate::Minimum
            | SimpleAggregate::Maximum
    ) {
        return Some("it calls an aggregate other than SUM, COUNT, MIN or MAX".to_owned());
    }
    let mut read = nodes(argument);
    read.extend(filter.as_deref().map(nodes).unwrap_or_default());
    read.into_iter()
        .find_map(|node| row_level_problem(node, of))
}

/// Why `node`, part of the argument or the filter, is not allowed in a row-level expression
/// over the dataset `of`.
fn row_level_problem(node: &Expression, of: &Name) -> Option<String> {
    match node {
        Expression::Reference(Reference {
            qualifier: None,
            name,
        }) => Some(format!(
            "the bare reference `{}`; a field of `of` is read as `of.field`",
            name.text()
        )),
        Expression::Reference(Reference {
            qualifier: Some(qualifier),
            name,
        }) if Name::of(qualifier) != *of => Some(format!(
            "`{}`.`{}` reads a dataset other than `of`",
            qualifier.text(),
            name.text()
        )),
        Expression::Aggregate { .. } => Some("an aggregate inside the aggregate".to_owned()),
        Expression::MetricAggregate(_) => Some("AGG(…)".to_owned()),
        Expression::UserAttribute(_) => Some("USER_ATTRIBUTE(…)".to_owned()),
        _ => None,
    }
}

/// The field references of `tree`, every one of them `of.field` once the shape is valid.
fn read_fields(tree: &Expression) -> Vec<&Reference> {
    nodes(tree)
        .into_iter()
        .filter_map(|node| match node {
            Expression::Reference(reference) => Some(reference),
            _ => None,
        })
        .collect()
}

/// The type of the field a reference of a valid `base_aggregate` reads in `base`.
fn field_type(
    fields: &Fields<'_, '_>,
    base: &DatasetOutline<'_>,
    reference: &Reference,
) -> Outcome<Typed> {
    fields
        .type_of(base, &Name::of(&reference.name))
        .cloned()
        .ok_or(Stop::ReportedElsewhere)
}
