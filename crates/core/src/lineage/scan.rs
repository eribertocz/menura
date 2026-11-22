//! The provenance of the columns of a `Scan` node: the catalog columns each reads, and the
//! model fields it stands for.

use std::collections::BTreeSet;

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::{CatalogSchemaDocument, RelationSchema};
use menura_model::expression::Reference;
use menura_model::internal_model::{Dataset, FieldPath, InternalModel, NormalizedName, Summary};
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::{ColumnName, ColumnPrefix};

use super::provenance::{CatalogColumn, NodeLineage, Provenance};
use super::references::{normalized_name, references};
use crate::compile::InternalError;

/// What the lineage of a plan reads its datasets and relations from.
#[derive(Debug, Clone, Copy)]
pub(super) struct Sources<'input> {
    /// The model.
    pub(super) model: &'input InternalModel,
    /// The admitted catalog schema.
    pub(super) catalog: &'input CatalogSchemaDocument,
}

impl Sources<'_> {
    /// The lineage of the `Scan` `node` of `dataset`, whose columns are under `prefix` when it
    /// is the right input of a `Join` (E6-4.12), and whose `row_identity` column reads nothing.
    ///
    /// # Errors
    ///
    /// An [`InternalError`] for a dataset, relation, field or catalog column the plan names and
    /// the model or the catalog does not have.
    pub(super) fn scan(
        &self,
        node: &PlanNode,
        dataset: &NormalizedName,
        row_identity: Option<&ColumnName>,
        prefix: Option<&ColumnPrefix>,
    ) -> Result<NodeLineage, InternalError> {
        let found = self
            .model
            .dataset(dataset)
            .ok_or_else(|| unknown("dataset", dataset.as_str()))?;
        let relation = self
            .catalog
            .relations
            .iter()
            .find(|relation| relation.source == found.source)
            .ok_or_else(|| unknown("relation", &found.source))?;
        let summary = self.summary_of(dataset);
        let source_prefix = source_prefix(found);
        let mut lineage = NodeLineage::default();
        for column in node.schema().columns() {
            let name = column.name.as_str();
            if row_identity == Some(&column.name) {
                lineage
                    .columns
                    .insert(name.to_owned(), Provenance::default());
                continue;
            }
            let local = match prefix {
                Some(prefix) => prefix
                    .field(name)
                    .ok_or_else(|| unknown("joined column", name))?,
                None => name,
            };
            let provenance = match local.strip_prefix(source_prefix.as_str()) {
                Some(catalog_name) => source_column(relation, catalog_name)?,
                None => field(found, relation, summary, local)?,
            };
            lineage.columns.insert(name.to_owned(), provenance);
        }
        lineage
            .shared
            .datasets
            .insert(summary.map_or_else(|| dataset.clone(), |summary| summary.of.clone()));
        lineage.shared.summary = summary.map(|summary| summary.dataset.clone());
        Ok(lineage)
    }

    /// The summary whose summary dataset is `dataset`, if any.
    fn summary_of(&self, dataset: &NormalizedName) -> Option<&Summary> {
        self.model
            .parts()
            .summaries
            .iter()
            .find(|summary| &summary.dataset == dataset)
    }
}

/// The provenance of the field `name` of `dataset`: the catalog columns of `relation` its
/// expression reads and, for a summary field, the base fields it stands for (E5-3.5a).
fn field(
    dataset: &Dataset,
    relation: &RelationSchema,
    summary: Option<&Summary>,
    name: &str,
) -> Result<Provenance, InternalError> {
    let field = dataset
        .field(&NormalizedName::new(name))
        .ok_or_else(|| unknown("field", name))?;
    let expression = field
        .expression
        .as_ref()
        .ok_or_else(|| unknown("field expression", name))?;
    let mut provenance = Provenance::default();
    for reference in references(expression) {
        let column = catalog_column(relation, reference.name.text())?;
        provenance.catalog_columns.insert(column);
    }
    match summary {
        Some(summary) => provenance.fields = summary_base_fields(summary, &field.name),
        None => {
            provenance.fields.insert(FieldPath {
                dataset: dataset.name.clone(),
                field: field.name.clone(),
            });
        }
    }
    Ok(provenance)
}

/// The fields of the base dataset that the summary field `field` stands for: the `base` of
/// its dimension, or the fields the `base_aggregate` of its measure reads (SUM-2).
fn summary_base_fields(summary: &Summary, field: &NormalizedName) -> BTreeSet<FieldPath> {
    let base_field = |name: NormalizedName| FieldPath {
        dataset: summary.of.clone(),
        field: name,
    };
    if let Some(dimension) = summary
        .dimensions
        .iter()
        .find(|dimension| &dimension.field == field)
    {
        return std::iter::once(base_field(dimension.base.clone())).collect();
    }
    summary
        .measures
        .iter()
        .filter(|measure| &measure.field == field)
        .filter_map(|measure| measure.base_aggregate.as_ref())
        .flat_map(references)
        .map(|reference: &Reference| base_field(normalized_name(&reference.name)))
        .collect()
}

/// The provenance of the `Scan` column that names the catalog column `catalog_name`.
fn source_column(
    relation: &RelationSchema,
    catalog_name: &str,
) -> Result<Provenance, InternalError> {
    let mut provenance = Provenance::default();
    provenance
        .catalog_columns
        .insert(catalog_column(relation, catalog_name)?);
    Ok(provenance)
}

/// The column of `relation` named `name`, compared as E0D-1.5 states, in the catalog's
/// spelling (PLN-4.5).
fn catalog_column(relation: &RelationSchema, name: &str) -> Result<CatalogColumn, InternalError> {
    let wanted = normalize_unquoted_identifier(name);
    relation
        .columns
        .iter()
        .find(|column| normalize_unquoted_identifier(&column.name) == wanted)
        .map(|column| CatalogColumn {
            source: relation.source.clone(),
            column: column.name.clone(),
        })
        .ok_or_else(|| unknown("catalog column", name))
}

/// The prefix of the `Scan` columns of `dataset` that name a catalog column rather than a
/// field: the shortest `menura.source` followed by one or more `.` that no field name starts
/// with (issue #264).
fn source_prefix(dataset: &Dataset) -> String {
    let mut prefix = String::from("menura.source.");
    while dataset
        .fields
        .iter()
        .any(|field| field.name.as_str().starts_with(&prefix))
    {
        prefix.push('.');
    }
    prefix
}

/// The defect of a plan that names a `kind` called `name` that its inputs do not have.
fn unknown(kind: &str, name: &str) -> InternalError {
    InternalError::new(
        "core",
        format!("the plan reads the {kind} `{name}`, which its inputs do not have"),
    )
}
