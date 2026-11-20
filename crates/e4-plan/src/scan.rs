//! The `Scan` of the home dataset (E4-4 step 1).

use menura_model::internal_model::{Dataset, NormalizedName, ValueType};
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode, SchemaColumn};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use crate::error::{InvariantViolation, PlanError};

/// The `Scan` of `home` that projects the fields of `read`, in the order of the dataset's
/// fields, each column named after the field's normalized name (issue #251).
///
/// # Errors
///
/// An internal error for a field that `home` does not have, or that has no type.
pub(crate) fn scan(home: &Dataset, read: &[NormalizedName]) -> Result<PlanNode, PlanError> {
    let origin = Origin::Model(home.location.clone());
    let fail = |violation| PlanError::invariant(violation, &origin);
    if read.iter().any(|name| home.field(name).is_none()) {
        return Err(fail(InvariantViolation::ForeignField));
    }
    let columns = home
        .fields
        .iter()
        .filter(|field| read.contains(&field.name))
        .map(|field| {
            let typed = field
                .typed
                .as_ref()
                .ok_or_else(|| fail(InvariantViolation::UntypedField))?;
            let ValueType::Logical(logical_type) = &typed.value_type else {
                return Err(fail(InvariantViolation::UntypedField));
            };
            Ok(SchemaColumn {
                name: ColumnName::new(field.name.as_str()),
                logical_type: logical_type.clone(),
                nullable: typed.nullable,
            })
        })
        .collect::<Result<Vec<_>, PlanError>>()?;
    Ok(PlanNode::new(
        Operation::Scan {
            dataset: home.name.clone(),
        },
        OutputSchema::new(columns),
        origin.clone(),
    ))
}
