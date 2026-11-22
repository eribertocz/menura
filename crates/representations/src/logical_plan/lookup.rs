//! The values a `Lookup` node carries: how a result row selects a row of the context query,
//! and which of its columns it attaches (E4-3.16, E4-8.7).

use menura_model::expression::Literal;

use crate::names::ColumnName;

/// One key of the mapping of a `Lookup`: how a result row and a row of the context query are
/// matched on one key (E4-8.7). Every key of the mapping must match for a row to be selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupKey {
    /// The `result` column equals the `value` column, NULLs compared equal (E4-5.0a).
    Equal {
        /// The key column of the result rows.
        result: ColumnName,
        /// The key column of the context query.
        value: ColumnName,
    },
}

/// One column a `Lookup` attaches to each result row: a component of the base metric in the
/// context query (E4-8.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupColumn {
    /// The component column, as the context query outputs it.
    pub value: ColumnName,
    /// The column it is attached under.
    pub name: ColumnName,
    /// The component's value over no rows (E4-5.0b), taken by a result row with no match:
    /// `0` for `COUNT`; `None` is NULL.
    pub empty_value: Option<Literal>,
}
