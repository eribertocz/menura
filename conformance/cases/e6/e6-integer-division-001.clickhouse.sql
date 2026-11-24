WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "region" AS "REGION",
      "team_id" AS "TEAM_ID"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "QUANTITY",
      "REGION",
      "TEAM_ID"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("TEAM_ID")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_1_component_2"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "region",
      CAST(CASE WHEN toFloat64("metric_1_component_2") = 0 THEN NULL ELSE toFloat64("metric_1_component_1") / toFloat64("metric_1_component_2") END AS Nullable(Float64)) AS "team_per_quantity"
    FROM "menura_3_aggregate"
  )
SELECT
  "region",
  "team_per_quantity"
FROM "menura_4_project"
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