//! The step of a `Lookup` node (E4-3.16, E4-8.7, E6-4.14): the components of a context query
//! attached to the result rows.
//!
//! After the steps of the result rows and of the context query (E6-4.12) comes one `present`
//! step, the context rows with a presence column, TRUE on every row. The result rows are then
//! joined to it with a `LEFT JOIN` on the keys of the mapping, compared with
//! `IS NOT DISTINCT FROM` (NULLs equal, E4-5.0a), or `ON TRUE` when the context query has no
//! dimension and so one row (E4-5.2). Each result row keeps its columns and takes each attached
//! component from its matched row, or the component's empty-input value when it has none
//! (E4-5.0b), written as a literal (E6-4.11). The SQL shape is fixed by golden files only
//! (E6-4.14).

use menura_representations::logical_plan::{LookupColumn, LookupKey, Operation, PlanNode};

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::steps::{StepWriter, column_type, error};
use crate::stitch::{has_column, presence_column, present_rows};
use crate::values::literal_sql;

/// The alias of the result rows.
const RESULT: &str = "\"menura_result\"";

/// The alias of the context rows.
const CONTEXT: &str = "\"menura_context\"";

impl<'input> StepWriter<'input> {
    /// The body of `node`, a `Lookup`; any other node is not one this function generates.
    pub(crate) fn lookup_node(
        &mut self,
        node: &'input PlanNode,
    ) -> Result<SelectBody, GenerateError> {
        let Operation::Lookup {
            result,
            value,
            mapping,
            columns,
        } = node.operation()
        else {
            return Err(error(InvariantViolation::NodeOutsideSlice, node));
        };
        check_output(node, result, columns)?;
        let result_name = self.write(result)?.name;
        let value_name = self.write(value)?.name;
        self.parameters.start_node(node.origin());
        let presence = presence_column(std::slice::from_ref(value.as_ref()));
        let present = self.push_step("present", present_rows(value, &value_name, &presence));
        let mut items: Vec<String> = result
            .schema()
            .columns()
            .iter()
            .map(|column| {
                let name = quote(column.name.as_str());
                format!("{RESULT}.{name} AS {name}")
            })
            .collect();
        for column in columns {
            items.push(attached_item(node, value, &presence, column)?);
        }
        let condition = match_condition(node, result, value, mapping)?;
        Ok(SelectBody {
            items,
            from: format!("{result_name} AS {RESULT}"),
            clauses: vec![format!("LEFT JOIN {present} AS {CONTEXT} ON {condition}")],
        })
    }
}

/// Checks that the schema of `node` is the columns of `result`, then those of `columns`.
fn check_output(
    node: &PlanNode,
    result: &PlanNode,
    columns: &[LookupColumn],
) -> Result<(), GenerateError> {
    let expected: Vec<&str> = result
        .schema()
        .columns()
        .iter()
        .map(|column| column.name.as_str())
        .chain(columns.iter().map(|column| column.name.as_str()))
        .collect();
    let schema = node.schema().columns();
    let same = schema.len() == expected.len()
        && schema
            .iter()
            .zip(&expected)
            .all(|(column, name)| column.name.as_str() == *name);
    if same {
        Ok(())
    } else {
        Err(error(InvariantViolation::UnknownColumn, node))
    }
}

/// The join condition of the mapping: every key, joined with `AND`, or `TRUE` without keys.
fn match_condition(
    node: &PlanNode,
    result: &PlanNode,
    value: &PlanNode,
    mapping: &[LookupKey],
) -> Result<String, GenerateError> {
    if mapping.is_empty() {
        return Ok("TRUE".to_owned());
    }
    let conditions = mapping
        .iter()
        .map(|key| match key {
            LookupKey::Equal {
                result: result_key,
                value: value_key,
            } => {
                if !has_column(result, result_key) || !has_column(value, value_key) {
                    return Err(error(InvariantViolation::UnknownColumn, node));
                }
                Ok(format!(
                    "{RESULT}.{} IS NOT DISTINCT FROM {CONTEXT}.{}",
                    quote(result_key.as_str()),
                    quote(value_key.as_str())
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(conditions.join(" AND "))
}

/// The select item of `column`: the value of the matched context row, or the empty-input
/// value cast to the column type when no row matches (`NULL` when it has none).
fn attached_item(
    node: &PlanNode,
    value: &PlanNode,
    presence: &str,
    column: &LookupColumn,
) -> Result<String, GenerateError> {
    if !has_column(value, &column.value) {
        return Err(error(InvariantViolation::UnknownColumn, node));
    }
    let empty = match &column.empty_value {
        Some(literal) => format!(
            "CAST({} AS {})",
            literal_sql(literal),
            column_type(node, column.name.as_str())?
        ),
        None => "NULL".to_owned(),
    };
    Ok(format!(
        "CASE WHEN {CONTEXT}.{} THEN {CONTEXT}.{} ELSE {empty} END AS {}",
        quote(presence),
        quote(column.value.as_str()),
        quote(column.name.as_str())
    ))
}
