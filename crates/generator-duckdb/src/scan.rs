//! The step of a `Scan` node (E4-3.1): the fields and source columns of a dataset, read from
//! its catalog relation.

use menura_contracts::types::catalog::RelationSchema;
use menura_model::internal_model::{Dataset, NormalizedName};
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::{ColumnName, ColumnPrefix};

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::identifiers::{Source, classify_source, quote, source_sql};
use crate::source_columns::{
    empty_scan_item, source_column_item, source_column_name, source_prefix,
};
use crate::steps::{StepWriter, error};

/// What a `Scan` reads besides its dataset: the prefix of its columns and its row identity.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScanRole<'input> {
    /// The `right_prefix` of the `Join` whose right input it is (E6-4.12).
    pub(crate) column_prefix: Option<&'input ColumnPrefix>,
    /// The synthesized row identity of a home `Scan` (E4-4.0f).
    pub(crate) row_identity: Option<&'input ColumnName>,
}

impl<'input> StepWriter<'input> {
    /// Each schema column is the expression of the field of the same name, over the columns of
    /// the catalog relation whose `source` is the dataset's, character for character (CAT-4),
    /// a query `source` included (E6-4.3); a schema column that names a catalog column is that
    /// column, in the catalog's spelling (PLN-2.2). In the `Scan` of a join instance, that
    /// name follows the `right_prefix` of its `Join` (E6-4.12). The row identity is a row
    /// number over the relation (E4-4.0f). A `Scan` with no column selects one constant, so
    /// that the rows can still be counted.
    pub(crate) fn scan(
        &mut self,
        node: &'input PlanNode,
        dataset: &NormalizedName,
        role: ScanRole<'input>,
    ) -> Result<SelectBody, GenerateError> {
        let model = self.environment.model;
        let dataset = model
            .dataset(dataset)
            .ok_or_else(|| error(InvariantViolation::UnknownDataset, node))?;
        let source: Source<'_> = classify_source(&dataset.source)
            .ok_or_else(|| error(InvariantViolation::MalformedSource, node))?;
        let relation = self
            .environment
            .catalog
            .relations
            .iter()
            .find(|relation| relation.source == dataset.source)
            .ok_or_else(|| error(InvariantViolation::UnknownRelation, node))?;
        self.parameters.start_node(node.origin());
        let prefix = source_prefix(dataset);
        let mut items = Vec::new();
        for column in node.schema().columns() {
            let name = column.name.as_str();
            if role.row_identity == Some(&column.name) {
                items.push(format!("ROW_NUMBER() OVER () AS {}", quote(name)));
                continue;
            }
            let local = local_name(role.column_prefix, name)
                .ok_or_else(|| error(InvariantViolation::UnknownField, node))?;
            let item = match source_column_name(&prefix, local) {
                Some(catalog_name) => source_column_item(relation, catalog_name, name)
                    .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?,
                None => self.field_item(node, dataset, relation, local, name)?,
            };
            items.push(item);
        }
        if items.is_empty() {
            items.push(empty_scan_item(&prefix));
        }
        Ok(SelectBody {
            items,
            from: source_sql(&source),
            clauses: Vec::new(),
        })
    }

    /// The select item of the `Scan` column `name` of `dataset`: the expression of its field
    /// `field_name` over the columns of `relation`.
    fn field_item(
        &mut self,
        node: &'input PlanNode,
        dataset: &Dataset,
        relation: &'input RelationSchema,
        field_name: &str,
        name: &str,
    ) -> Result<String, GenerateError> {
        let field = dataset
            .fields
            .iter()
            .find(|field| field.name.as_str() == field_name)
            .ok_or_else(|| error(InvariantViolation::UnknownField, node))?;
        let expression = field
            .expression
            .as_ref()
            .ok_or_else(|| error(InvariantViolation::FieldWithoutExpression, node))?;
        let value = self.expression(node, Columns::Catalog(relation), expression)?;
        Ok(format!("{value} AS {}", quote(name)))
    }
}

/// The name of the `Scan` column `name` within its dataset: `name` itself in the home `Scan`,
/// and the text after `prefix` in the `Scan` of a join instance; `None` when such a column
/// lacks the prefix.
fn local_name<'name>(prefix: Option<&ColumnPrefix>, name: &'name str) -> Option<&'name str> {
    match prefix {
        None => Some(name),
        Some(prefix) => prefix.field(name),
    }
}
