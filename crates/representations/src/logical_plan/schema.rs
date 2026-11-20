//! The output schema of a plan node (E4-3): names, logical types and nullability.

use menura_contracts::types::common::LogicalType;

use crate::names::ColumnName;

/// One column a plan node produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaColumn {
    /// Its name.
    pub name: ColumnName,
    /// Its logical type (CAT-4.1).
    pub logical_type: LogicalType,
    /// Whether it can be NULL.
    pub nullable: bool,
}

/// The columns a plan node produces, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSchema(Vec<SchemaColumn>);

impl OutputSchema {
    /// The schema of `columns`, in order.
    #[must_use]
    pub fn new(columns: Vec<SchemaColumn>) -> Self {
        Self(columns)
    }

    /// The columns, in order.
    #[must_use]
    pub fn columns(&self) -> &[SchemaColumn] {
        &self.0
    }
}
