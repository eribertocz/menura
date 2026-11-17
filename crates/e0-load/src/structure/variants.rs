//! The compiled variant of every expression of the selected model (E0B-3.3, E0B-3.4, section
//! 4.5).

use super::datatype::declared_datatype;
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

/// Picks the compiled variant of every field and metric of a structurally valid `model`, located
/// at `pointer`: each dataset's fields, then its dataset-scoped metrics (development format),
/// then the model metrics. The caller sorts the expressions and the warnings.
pub(crate) fn pick_variants<'document>(
    model: &'document Node,
    pointer: &JsonPointer,
) -> Variants<'document> {
    let mut variants = Variants {
        expressions: Vec::new(),
        warnings: Vec::new(),
    };
    let dataset_list = pointer.member("datasets");
    for (index, dataset) in items(member(model, "datasets")) {
        let dataset_pointer = dataset_list.item(index);
        for (list, kind) in [
            ("fields", ElementKind::Field),
            ("metrics", ElementKind::DatasetMetric),
        ] {
            let list_pointer = dataset_pointer.member(list);
            for (element_index, element) in items(member(dataset, list)) {
                variants.add(kind, element, list_pointer.item(element_index));
            }
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
        let ansi_sql = self.dialect_variants(expression, &expression_pointer);
        let variant = if let Some(sql) = text(member(expression, "sql")) {
            CompiledVariant::OssieSql2026(sql)
        } else if let [sql] = ansi_sql.as_slice() {
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
        let (datatype, warning) = declared_datatype(element, &pointer);
        self.warnings.extend(warning);
        self.expressions.push(ElementExpression {
            kind,
            pointer,
            variant,
            datatype,
        });
    }

    /// The text of every `ANSI_SQL` variant of `expression`, with a `W0B-01` warning for each
    /// variant in another dialect.
    fn dialect_variants(
        &mut self,
        expression: &'document Node,
        expression_pointer: &JsonPointer,
    ) -> Vec<&'document str> {
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
        ansi_sql
    }
}

/// The items of a sequence node with their indices; nothing for any other node.
pub(crate) fn items(node: Option<&Node>) -> impl Iterator<Item = (usize, &Node)> {
    let items: &[Node] = match node.map(|node| &node.value) {
        Some(Value::Sequence(items)) => items,
        _ => &[],
    };
    items.iter().enumerate()
}

pub(crate) fn text(node: Option<&Node>) -> Option<&str> {
    match node.map(|node| &node.value) {
        Some(Value::String(text)) => Some(text),
        _ => None,
    }
}
