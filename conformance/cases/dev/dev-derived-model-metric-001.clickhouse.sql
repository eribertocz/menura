WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "status" AS "STATUS"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "STATUS" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(coalesce(count(), 0), 'Nullable(Int64)') AS "metric_1_component_2"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "status",
      CAST(CASE WHEN toFloat64("metric_1_component_2") = 0 THEN NULL ELSE toFloat64("metric_1_component_1") / toFloat64("metric_1_component_2") END AS Nullable(Float64)) AS "average_sale"
    FROM "menura_2_aggregate"
  )
SELECT
  "status",
  "average_sale"
FROM "menura_3_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1,
  use_variant_as_common_type = 0