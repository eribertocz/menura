//! The `Scan` of the home dataset (E4-4.0a) and of each join instance (E4-4.0c).

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::{CatalogSchemaDocument, RelationSchema};
use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_model::internal_model::{Dataset, NormalizedName, ValueType};
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode, SchemaColumn};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;

use crate::columns::source_prefix;
use crate::error::{InvariantViolation, PlanError};

/// The `Scan` of `home` that projects the columns of `read`: its fields, in the order of the
/// dataset's fields, each named after the field's normalized name (issue #251), then the
/// source columns, in first-read order, each with the type and nullability `catalog` gives it
/// (CAT-4, issue #264), then the synthesized row `identity` when there is one (E4-4.0f).
///
/// # Errors
///
/// An internal error for a field that `home` does not have or that has no type, and for a
/// source column that the catalog relation of `home` does not have.
pub(crate) fn scan(
    home: &Dataset,
    read: &[NormalizedName],
    catalog: &CatalogSchemaDocument,
    identity: Option<&ColumnName>,
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
    let mut columns = field_columns(home, &field_names, ColumnName::new).map_err(fail)?;
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
    if let Some(identity) = identity {
        columns.push(identity_column(identity.clone()));
    }
    Ok(PlanNode::new(
        Operation::Scan {
            dataset: home.name.clone(),
            row_identity: identity.cloned(),
        },
        OutputSchema::new(columns),
        origin.clone(),
    ))
}

/// The `Scan` of the join instance of `dataset` whose columns are under `prefix`: the fields
/// whose column `read` names, in the order of the dataset's fields (E6-4.12).
///
/// # Errors
///
/// An internal error for a field without a type.
pub(crate) fn instance_scan(
    dataset: &Dataset,
    prefix: &ColumnPrefix,
    read: &[NormalizedName],
) -> Result<PlanNode, PlanError> {
    let origin = Origin::Model(dataset.location.clone());
    let fields: Vec<&NormalizedName> = dataset
        .fields
        .iter()
        .map(|field| &field.name)
        .filter(|name| {
            let column = prefix.column(name.as_str());
            read.iter().any(|wanted| wanted.as_str() == column.as_str())
        })
        .collect();
    let columns = field_columns(dataset, &fields, |name| prefix.column(name))
        .map_err(|violation| PlanError::invariant(violation, &origin))?;
    Ok(PlanNode::new(
        Operation::Scan {
            dataset: dataset.name.clone(),
            row_identity: None,
        },
        OutputSchema::new(columns),
        origin,
    ))
}

/// The synthesized row identity `name`: a 64-bit integer, never NULL (E4-4.0f).
fn identity_column(name: ColumnName) -> SchemaColumn {
    SchemaColumn {
        name,
        logical_type: LogicalType::Integer {
            bits: IntegerBits::SixtyFour,
            signed: true,
        },
        nullable: false,
    }
}

/// The `Scan` columns of the fields of `dataset` that `read` names, in the dataset's order,
/// each named by `column_name` from the field's name.
fn field_columns(
    dataset: &Dataset,
    read: &[&NormalizedName],
    column_name: impl Fn(&str) -> ColumnName,
) -> Result<Vec<SchemaColumn>, InvariantViolation> {
    dataset
        .fields
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
                name: column_name(field.name.as_str()),
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
