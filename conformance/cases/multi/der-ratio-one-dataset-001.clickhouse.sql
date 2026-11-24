WITH
  "menura_1_scan" AS (
    SELECT
      "billed" AS "BILLED",
      "collected" AS "COLLECTED"
    FROM "shop"."invoices"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(sumOrNull("COLLECTED") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1",
      CAST(sumOrNull("BILLED") AS Nullable(Decimal(38, 2))) AS "metric_1_component_2"
    FROM "menura_1_scan"
  ),
  "menura_3_project" AS (
    SELECT
      CAST(CASE WHEN toFloat64("metric_1_component_2") = 0 THEN NULL ELSE toFloat64("metric_1_component_1") / toFloat64("metric_1_component_2") END AS Nullable(Float64)) AS "collection_rate"
    FROM "menura_2_aggregate"
  )
SELECT
  "collection_rate"
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