//! `E_NAME_NOT_FOUND` (row 3 of E2-5), with its alternatives (PLN-5.4).

use menura_model::expression::Reference;
use menura_representations::origin::QueryLocation;

use crate::alternatives::ExpectedKind;
use crate::error::{BindCode, BindDiagnostic, BindError};
use crate::names::{expression_reference_names, written_expression_reference};
use crate::resolver::{NameScope, Resolver};

/// `E_NAME_NOT_FOUND` for the expression reference `reference`, read in `scope` at a
/// position that expects `expected`, with its alternatives (PLN-5.4).
pub(crate) fn expression_not_found(
    reference: &Reference,
    expected: ExpectedKind,
    scope: NameScope<'_>,
    origin: &QueryLocation,
    resolver: &Resolver<'_>,
) -> BindError {
    let names = Some(expression_reference_names(reference));
    let alternatives = resolver.alternatives(expected, scope, names);
    not_found(
        &written_expression_reference(reference),
        "field or metric",
        origin,
        alternatives,
    )
}

/// `E_NAME_NOT_FOUND` for the name `written`, which should name a `kind`, carrying
/// `alternatives` (PLN-5.4). The message and the alternatives are the same whether the name is
/// unknown or unauthorized (E2-4.4, PLN-5.2, PLN-5.4a).
pub(crate) fn not_found(
    written: &str,
    kind: &str,
    origin: &QueryLocation,
    alternatives: Vec<String>,
) -> BindError {
    let message = format!(
        "`{written}` does not name a {kind} of the model; check the name and how it is written"
    );
    BindDiagnostic::new(BindCode::NameNotFound, message, origin)
        .with_alternatives(alternatives)
        .into()
}
