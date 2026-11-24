WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "segment" AS "SEGMENT"
    FROM "shop"."customers"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "SEGMENT" AS "dimension_1",
      accurateCast(coalesce(count("CUSTOMER_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "segment",
      "metric_1_component_1" AS "customer_count"
    FROM "menura_2_aggregate"
  )
SELECT
  "segment",
  "customer_count"
FROM "menura_3_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1