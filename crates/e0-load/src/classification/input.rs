//! What classification reads from the substages before it, and the query error each of them
//! records for a metric it made unusable.

use super::output::QueryError;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::{ExtensionCode, Extensions};
use crate::pointer::JsonPointer;
use crate::references::References;
use crate::references::derived::DerivedMetrics;
use crate::structure::{CompiledVariant, ElementExpression, Structure};
use crate::typing::Typing;

/// The results of E0.c to E0.f that classification reads.
#[derive(Debug, Clone, Copy)]
pub struct ClassificationInput<'earlier> {
    /// The extensions E0.c validated: additivity declarations, and the metrics marked
    /// `derived` or `context`.
    pub extensions: &'earlier Extensions,
    /// The elements E0.d made unusable.
    pub references: &'earlier References,
    /// The derived metrics E0.d found, with their inputs, and the metrics `E0C-37` makes
    /// unusable.
    pub derived: &'earlier DerivedMetrics,
    /// The types E0.f found for the fields and the other metrics, and the elements it made
    /// unusable.
    pub typing: &'earlier Typing,
    /// The types E0.f found for the derived metrics, and the ones it made unusable.
    pub derived_typing: &'earlier Typing,
}

impl ClassificationInput<'_> {
    /// The query error of the metric at `pointer` when a substage before E0.g made it unusable,
    /// looked up in substage order: its expression (E0.b, then parsing), its references (E0.d),
    /// `E0C-37`, then typing (E0.f).
    pub(super) fn earlier_error(
        &self,
        structure: &Structure<'_>,
        pointer: &JsonPointer,
    ) -> Option<QueryError> {
        if let Some(expression) = structure
            .expressions
            .iter()
            .find(|expression| expression.pointer == *pointer)
            && let Some(error) = expression_error(expression)
        {
            return Some(error);
        }
        if let Some(element) = self
            .references
            .unusable
            .iter()
            .find(|element| element.pointer == *pointer)
        {
            return Some(QueryError::Reference(element.query_error));
        }
        if self.derived.unusable.contains(pointer) {
            return Some(QueryError::Extension(
                ExtensionCode::AggregateOutsideDerived,
            ));
        }
        [self.typing, self.derived_typing]
            .iter()
            .flat_map(|typing| &typing.unusable)
            .find(|element| element.pointer == *pointer)
            .map(|element| QueryError::Typing(element.query_error))
    }
}

/// The query error of an element whose expression cannot be used: no single `ANSI_SQL`
/// variant or an unknown `datatype` (E0.b), or an expression that does not parse.
fn expression_error(expression: &ElementExpression<'_>) -> Option<QueryError> {
    if let Some(code) = expression.query_error() {
        return Some(QueryError::Structure(code));
    }
    let text = match expression.variant {
        CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text) => text,
        CompiledVariant::Unusable => return None,
    };
    parse_element_expression(text, ExpressionPlace::Metric)
        .err()
        .map(|diagnostic| QueryError::Expression(diagnostic.code))
}
