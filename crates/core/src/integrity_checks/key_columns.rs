//! The columns of a relationship key in the plan of a check: the `Scan` column each key name
//! reads, its type, and the name of the result column that holds its value (CHK-2.6).

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::LogicalType;
use menura_model::internal_model::{Dataset, NormalizedName, ValueType};
use menura_representations::names::ColumnName;

use crate::compile::InternalError;

/// The stage named by the defects of the checks: the integrity-check generation of `core`.
pub(super) const STAGE: &str = "integrity_checks";

/// One key column of a relationship, as a `Scan` of its dataset reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KeyColumn {
    /// The `Scan` column: the field's normalized name, or the source column under the source
    /// prefix of the dataset (E0D-3.1, E6-4.12).
    pub(super) scan: ColumnName,
    /// The name of the result column that holds its value: the key's normalized name
    /// (CHK-2.6, E0H-1.1), for a field and for a source column alike.
    pub(super) result: String,
    /// Its logical type.
    pub(super) logical_type: LogicalType,
    /// Whether it can be NULL.
    pub(super) nullable: bool,
}

/// The key columns `names` of `dataset`, in order. A name resolves to the field of that name,
/// or else to the column of that name of the dataset's catalog relation (E0D-3.1).
pub(super) fn key_columns(
    dataset: &Dataset,
    names: &[NormalizedName],
    catalog: &CatalogSchemaDocument,
) -> Result<Vec<KeyColumn>, InternalError> {
    names
        .iter()
        .map(|name| key_column(dataset, name, catalog))
        .collect()
}

fn key_column(
    dataset: &Dataset,
    name: &NormalizedName,
    catalog: &CatalogSchemaDocument,
) -> Result<KeyColumn, InternalError> {
    if let Some(field) = dataset.field(name) {
        let typed = field
            .typed
            .as_ref()
            .ok_or_else(|| defect(dataset, name, "has no type"))?;
        let ValueType::Logical(logical_type) = &typed.value_type else {
            return Err(defect(dataset, name, "has the type of NULL"));
        };
        return Ok(KeyColumn {
            scan: ColumnName::new(name.as_str()),
            result: name.as_str().to_owned(),
            logical_type: logical_type.clone(),
            nullable: typed.nullable,
        });
    }
    let wanted = normalize_unquoted_identifier(name.as_str());
    let column = catalog
        .relations
        .iter()
        .find(|relation| relation.source == dataset.source)
        .and_then(|relation| {
            relation
                .columns
                .iter()
                .find(|column| normalize_unquoted_identifier(&column.name) == wanted)
        })
        .ok_or_else(|| defect(dataset, name, "is neither a field nor a source column"))?;
    Ok(KeyColumn {
        scan: ColumnName::new(&format!("{}{}", source_prefix(dataset), name.as_str())),
        result: name.as_str().to_owned(),
        logical_type: column.logical_type.clone(),
        nullable: column.nullable,
    })
}

/// The shortest `menura.source` followed by one or more `.` that no field name of `dataset`
/// starts with: the prefix under which a `Scan` of `dataset` reads a source column, as E4 and
/// E6 name it (E6-4.12).
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

fn defect(dataset: &Dataset, name: &NormalizedName, what: &str) -> InternalError {
    InternalError::new(
        STAGE,
        format!(
            "the key column {} of the dataset {} {what}",
            name.as_str(),
            dataset.name.as_str()
        ),
    )
}
