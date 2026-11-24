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
//! | `aggregate_functions_null_for_empty` | `1` | An aggregate over no rows is NULL, not the default value of its type. |
//! | `decimal_check_overflow` | `1` | Decimal arithmetic that does not fit its type raises an error instead of wrapping (E6-7.9). |
//! | `use_variant_as_common_type` | `0` | The branches of a `UNION ALL` (and of `if`, `CASE` and arrays) whose types differ have their common type, or the statement fails; never a `Variant` that aggregates and comparisons reject. The generator casts the branches it unites to one type, so no common type is inferred (E6-4.5, E6-4.6b). |

/// Every setting of the `SETTINGS` clause, as `name = value`, in a fixed order (E6-4.9).
pub(crate) const SETTINGS: [&str; 9] = [
    "join_use_nulls = 1",
    "transform_null_in = 0",
    "prefer_column_name_to_alias = 1",
    "enable_positional_arguments = 1",
    "short_circuit_function_evaluation = 'force_enable'",
    "enable_extended_results_for_datetime_functions = 1",
    "aggregate_functions_null_for_empty = 1",
    "decimal_check_overflow = 1",
    "use_variant_as_common_type = 0",
];
