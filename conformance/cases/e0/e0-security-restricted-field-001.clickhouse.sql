WITH
  "menura_1_scan" AS (
    SELECT
      "unit_cost" AS "COST_COPY"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "COST_COPY"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("COST_COPY")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_2_filter"
  ),
  "menura_4_project" AS (
    SELECT
      "metric_1_component_1" AS "total_cost"
    FROM "menura_3_aggregate"
  )
SELECT
  "total_cost"
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