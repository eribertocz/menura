WITH
  "menura_1_scan" AS (
    SELECT
      CASE "quantity" WHEN CAST('2.0' AS Decimal(2, 1)) THEN 'pair' ELSE 'other' END AS "PAIR_MARKER",
      "quantity" AS "QUANTITY"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "PAIR_MARKER" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "pair_marker",
      "metric_1_component_1" AS "total_quantity"
    FROM "menura_2_aggregate"
  )
SELECT
  "pair_marker",
  "total_quantity"
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