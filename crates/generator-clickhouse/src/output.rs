//! What E6 produces for a plan: the parts of the compiled plan that come from the SQL.

use menura_contracts::types::common::Parameter;
use menura_contracts::types::compiled_plan::ResultColumn;

/// The SQL of a plan, its bound parameters and its result schema (PLN-1).
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedSql {
    /// One read-only statement (E6-4.1, PLN-2.3).
    pub sql: String,
    /// The bound parameters, by position (`$1`, `$2`, …), in order of first appearance in
    /// `sql` (E6-4.2).
    pub parameters: Vec<Parameter>,
    /// The output columns, in output order (PLN-1).
    pub result_schema: Vec<ResultColumn>,
}
