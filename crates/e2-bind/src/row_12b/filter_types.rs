//! The first check of row 12b (E2-3.6): every filter is typed as in E0.f, and is boolean. The
//! first filter in query order with a typing problem (`E0F-03` to `E0F-08`) is reported at
//! that filter, with the E0.f code; a filter that is not boolean is `E0F-06`.

use menura_contracts::types::common::LogicalType;
use menura_expressions::typing::{
    Aggregates, ExpressionType, Outcome, Stop, Typed, TypingCode, TypingScope, describe,
    type_expression,
};
use menura_model::expression::Reference;
use menura_model::internal_model::{InternalModel, TypedValue, ValueType};
use menura_representations::bound_query::BoundFilter;

use crate::error::{BindCode, BindDiagnostic};
use crate::names::identifier_name;

/// The first filter of `filters` whose type is not a boolean.
pub(crate) fn check_filter_types(
    model: &InternalModel,
    filters: &[BoundFilter],
) -> Result<(), BindDiagnostic> {
    let resolve = |reference: &Reference| field_type(model, reference);
    let scope = TypingScope::new(&resolve, Aggregates::NotAllowed);
    for filter in filters {
        let problem = match type_expression(&filter.expression, &scope) {
            Ok(typed) => not_boolean(&typed),
            Err(Stop::Problem { code, message }) => Some((code, message)),
            Err(Stop::ReportedElsewhere) => None,
        };
        if let Some((code, message)) = problem {
            return Err(BindDiagnostic::new(
                BindCode::FilterType(code),
                format!("this filter is not valid: {message}"),
                &filter.origin,
            ));
        }
    }
    Ok(())
}

/// `E0F-06` when `typed` is neither boolean nor the literal `NULL` (E0F-6).
fn not_boolean(typed: &Typed) -> Option<(TypingCode, String)> {
    match typed.logical() {
        None | Some(LogicalType::Boolean) => None,
        Some(_) => Some((
            TypingCode::NotBoolean,
            format!("it must be boolean, not {}", describe(&typed.value_type)),
        )),
    }
}

/// The type E0.f gave the field `reference` names. Rows 3 to 12a resolved every reference of
/// a filter to a usable field, so any other reference is left to those checks.
fn field_type(model: &InternalModel, reference: &Reference) -> Outcome<Typed> {
    let qualifier = reference
        .qualifier
        .as_ref()
        .ok_or(Stop::ReportedElsewhere)?;
    model
        .dataset(&identifier_name(qualifier))
        .and_then(|dataset| dataset.field(&identifier_name(&reference.name)))
        .and_then(|field| field.typed.as_ref())
        .map(typed)
        .ok_or(Stop::ReportedElsewhere)
}

/// `value` as the typer writes it.
fn typed(value: &TypedValue) -> Typed {
    let value_type = match &value.value_type {
        ValueType::Null => ExpressionType::Null,
        ValueType::Logical(logical) => ExpressionType::Value(logical.clone()),
    };
    Typed {
        value_type,
        nullable: value.nullable,
    }
}
