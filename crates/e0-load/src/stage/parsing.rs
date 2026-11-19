//! The load warning of every field and metric whose compiled expression does not parse:
//! `W0F-01`, with the query error `E0F-01`, `E0F-02` or `E_DEFERRED_WINDOWED_METRIC` (E0-4
//! table, E0F-2.2). No substage reports these, so the stage function does, within E0.f, at the
//! element's `expression` like the other diagnostics of an expression.

use super::findings::Finding;
use crate::expressions::{ExpressionDiagnostic, ExpressionPlace, parse_element_expression};
use crate::pointer::{JsonPointer, resolve};
use crate::read::{Location, Node};
use crate::structure::{CompiledVariant, ElementExpression, ElementKind, Structure};

/// One `W0F-01` finding per field and metric of `structure` whose compiled variant does not
/// parse. An element E0.b already made unusable has no compiled variant to parse and gets none.
pub(super) fn parse_warnings(document: &Node, structure: &Structure<'_>) -> Vec<Finding> {
    structure
        .expressions
        .iter()
        .filter_map(|expression| parse_warning(document, expression))
        .collect()
}

fn parse_warning(document: &Node, expression: &ElementExpression<'_>) -> Option<Finding> {
    if !expression.is_usable() {
        return None;
    }
    let (CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text)) = expression.variant
    else {
        return None;
    };
    let place = match expression.kind {
        ElementKind::Field => ExpressionPlace::Field,
        ElementKind::Metric | ElementKind::DatasetMetric => ExpressionPlace::Metric,
    };
    let diagnostic = parse_element_expression(text, place).err()?;
    let (pointer, location) = expression_node(document, &expression.pointer);
    Some(Finding {
        code: ExpressionDiagnostic::WARNING,
        rejects: false,
        pointer,
        location,
        message: format!(
            "the expression `{text}` cannot be used ({}): {}; the element is unusable until \
             its expression is fixed",
            diagnostic.code.as_str(),
            diagnostic.error
        ),
    })
}

/// The `expression` member of the element at `element`, where the other substages locate the
/// diagnostics of an expression, with where it starts; the element itself when it has none.
fn expression_node(document: &Node, element: &JsonPointer) -> (JsonPointer, Location) {
    let member = element.member("expression");
    if let Some(node) = resolve(document, &member) {
        return (member, node.location);
    }
    let location = resolve(document, element).map_or(document.location, |node| node.location);
    (element.clone(), location)
}
