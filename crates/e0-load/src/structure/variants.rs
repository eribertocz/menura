//! The `ANSI_SQL` variant of every expression of the selected model (E0B-3.3, E0B-3.4).

use super::diagnostic::{StructureCode, StructureDiagnostic};
use super::output::{CompiledVariant, ElementExpression, ElementKind};
use super::selection::member;
use crate::pointer::JsonPointer;
use crate::read::{Node, Value};

/// The dialect that is compiled.
const ANSI_SQL: &str = "ANSI_SQL";

/// The expressions of every field and metric of `model`, with their warnings.
pub(crate) struct Variants<'document> {
    pub(crate) expressions: Vec<ElementExpression<'document>>,
    pub(crate) warnings: Vec<StructureDiagnostic>,
}

/// Picks the `ANSI_SQL` variant of every field and metric of a structurally valid `model`,
/// located at `pointer`. Fields come before metrics; the caller sorts the warnings.
pub(crate) fn pick_variants<'document>(
    model: &'document Node,
    pointer: &JsonPointer,
) -> Variants<'document> {
    let mut variants = Variants {
        expressions: Vec::new(),
        warnings: Vec::new(),
    };
    let datasets_pointer = pointer.member("datasets");
    for (index, dataset) in items(member(model, "datasets")) {
        let fields_pointer = datasets_pointer.item(index).member("fields");
        for (field_index, field) in items(member(dataset, "fields")) {
            let element = fields_pointer.item(field_index);
            variants.add(ElementKind::Field, field, element);
        }
    }
    let metrics_pointer = pointer.member("metrics");
    for (index, metric) in items(member(model, "metrics")) {
        variants.add(ElementKind::Metric, metric, metrics_pointer.item(index));
    }
    variants
}

impl<'document> Variants<'document> {
    fn add(&mut self, kind: ElementKind, element: &'document Node, pointer: JsonPointer) {
        let Some(expression) = member(element, "expression") else {
            return;
        };
        let expression_pointer = pointer.member("expression");
        let dialects_pointer = expression_pointer.member("dialects");
        let mut ansi_sql = Vec::new();
        for (index, variant) in items(member(expression, "dialects")) {
            match (
                text(member(variant, "dialect")),
                text(member(variant, "expression")),
            ) {
                (Some(ANSI_SQL), Some(sql)) => ansi_sql.push(sql),
                (Some(dialect), _) => self.warnings.push(StructureDiagnostic {
                    code: StructureCode::OtherDialect,
                    pointer: dialects_pointer.item(index),
                    location: variant.location,
                    message: format!("the {dialect} variant is kept and not compiled"),
                }),
                (None, _) => {}
            }
        }
        let variant = if let [sql] = ansi_sql.as_slice() {
            CompiledVariant::AnsiSql(sql)
        } else {
            self.warnings.push(StructureDiagnostic {
                code: StructureCode::NoSingleAnsiSql,
                pointer: expression_pointer,
                location: expression.location,
                message: format!(
                    "the expression has {} ANSI_SQL variants instead of one; the element is unusable",
                    ansi_sql.len()
                ),
            });
            CompiledVariant::Unusable
        };
        self.expressions.push(ElementExpression {
            kind,
            pointer,
            variant,
        });
    }
}

/// The items of a sequence node with their indices; nothing for any other node.
fn items(node: Option<&Node>) -> impl Iterator<Item = (usize, &Node)> {
    let items: &[Node] = match node.map(|node| &node.value) {
        Some(Value::Sequence(items)) => items,
        _ => &[],
    };
    items.iter().enumerate()
}

fn text(node: Option<&Node>) -> Option<&str> {
    match node.map(|node| &node.value) {
        Some(Value::String(text)) => Some(text),
        _ => None,
    }
}
