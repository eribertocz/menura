//! The `SETTINGS` clause that ends every statement (E6-7.8): it fixes each server setting the
//! meaning of the generated SQL depends on, so that no session or server default changes a
//! result (E6-4.10).
//!
//! | Setting | Value | Why |
//! |---|---|---|
//! | `join_use_nulls` | `1` | The unmatched side of an outer join is NULL, not the default value of its type. |
//! | `transform_null_in` | `0` | `IN` never treats NULL as equal to NULL. |
//! | `prefer_column_name_to_alias` | `1` | A name reads the input column, never a select alias of the same name. |
//! | `enable_positional_arguments` | `1` | `GROUP BY 1, 2` reads select items by position. |
//! | `short_circuit_function_evaluation` | `'force_enable'` | A `CASE` branch is evaluated only for the rows that reach it, so a division or `throwIf` guarded by a condition never runs on the other rows (E6-4.6, E6-7.5). |
//! | `enable_extended_results_for_datetime_functions` | `1` | The truncation functions return `Date32` and `DateTime64`, whose range holds every value, not `Date` and `DateTime` (E6-7.6). |
//! | `aggregate_functions_null_for_empty` | `0` | Aggregates keep the names the generator writes; their NULL over no value is written explicitly. |

/// Every setting of the `SETTINGS` clause, as `name = value`, in a fixed order (E6-4.9).
pub(crate) const SETTINGS: [&str; 7] = [
    "join_use_nulls = 1",
    "transform_null_in = 0",
    "prefer_column_name_to_alias = 1",
    "enable_positional_arguments = 1",
    "short_circuit_function_evaluation = 'force_enable'",
    "enable_extended_results_for_datetime_functions = 1",
    "aggregate_functions_null_for_empty = 0",
];
