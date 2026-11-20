//! The `Scan` of the home dataset (E4-4 step 1).

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::{CatalogSchemaDocument, RelationSchema};
use menura_model::internal_model::{Dataset, NormalizedName, ValueType};
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode, SchemaColumn};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use crate::columns::source_prefix;
use crate::error::{InvariantViolation, PlanError};

/// The `Scan` of `home` that projects the columns of `read`: its fields, in the order of the
/// dataset's fields, each named after the field's normalized name (issue #251), then the
/// source columns, in first-read order, each with the type and nullability `catalog` gives it
/// (CAT-4, issue #264).
///
/// # Errors
///
/// An internal error for a field that `home` does not have or that has no type, and for a
/// source column that the catalog relation of `home` does not have.
pub(crate) fn scan(
    home: &Dataset,
    read: &[NormalizedName],
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, PlanError> {
    let origin = Origin::Model(home.location.clone());
    let fail = |violation| PlanError::invariant(violation, &origin);
    let prefix = source_prefix(home);
    let (source_names, field_names): (Vec<&NormalizedName>, Vec<&NormalizedName>) = read
        .iter()
        .partition(|name| name.as_str().starts_with(&prefix));
    if field_names.iter().any(|name| home.field(name).is_none()) {
        return Err(fail(InvariantViolation::ForeignField));
    }
    let mut columns = field_columns(home, &field_names).map_err(fail)?;
    if !source_names.is_empty() {
        let relation = catalog
            .relations
            .iter()
            .find(|relation| relation.source == home.source)
            .ok_or_else(|| fail(InvariantViolation::UnknownRelation))?;
        for name in source_names {
            if columns
                .iter()
                .any(|column| column.name.as_str() == name.as_str())
            {
                continue;
            }
            let catalog_name = &name.as_str()[prefix.len()..];
            columns.push(source_column(relation, name, catalog_name).map_err(fail)?);
        }
    }
    Ok(PlanNode::new(
        Operation::Scan {
            dataset: home.name.clone(),
        },
        OutputSchema::new(columns),
        origin.clone(),
    ))
}

/// The `Scan` columns of the fields of `home` that `read` names, in the dataset's order.
fn field_columns(
    home: &Dataset,
    read: &[&NormalizedName],
) -> Result<Vec<SchemaColumn>, InvariantViolation> {
    home.fields
        .iter()
        .filter(|field| read.contains(&&field.name))
        .map(|field| {
            let typed = field
                .typed
                .as_ref()
                .ok_or(InvariantViolation::UntypedField)?;
            let ValueType::Logical(logical_type) = &typed.value_type else {
                return Err(InvariantViolation::UntypedField);
            };
            Ok(SchemaColumn {
                name: ColumnName::new(field.name.as_str()),
                logical_type: logical_type.clone(),
                nullable: typed.nullable,
            })
        })
        .collect()
}

/// The `Scan` column `name` of the column of `relation` that `catalog_name` names: the column
/// whose name, normalized as E0D-1.1, equals `catalog_name` normalized the same way (E0D-1.5;
/// CAT-8.2 makes it unique).
fn source_column(
    relation: &RelationSchema,
    name: &NormalizedName,
    catalog_name: &str,
) -> Result<SchemaColumn, InvariantViolation> {
    let wanted = normalize_unquoted_identifier(catalog_name);
    let column = relation
        .columns
        .iter()
        .find(|column| normalize_unquoted_identifier(&column.name) == wanted)
        .ok_or(InvariantViolation::UnknownSourceColumn)?;
    Ok(SchemaColumn {
        name: ColumnName::new(name.as_str()),
        logical_type: column.logical_type.clone(),
        nullable: column.nullable,
    })
}
