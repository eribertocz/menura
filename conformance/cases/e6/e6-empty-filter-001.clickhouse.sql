WITH
  "menura_1_scan" AS (
    SELECT
      TRUE AS "menura.source."
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      TRUE AS "menura.row"
    FROM "menura_1_scan"
    WHERE {p1:Bool}
  ),
  "menura_3_aggregate" AS (
    SELECT
      accurateCast(coalesce(count(), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_2_filter"
  ),
  "menura_4_project" AS (
    SELECT
      "metric_1_component_1" AS "order_count"
    FROM "menura_3_aggregate"
  )
SELECT
  "order_count"
FROM "menura_4_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1