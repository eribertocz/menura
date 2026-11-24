WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "channel" AS "CHANNEL",
      "quantity" AS "QUANTITY"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CHANNEL",
      "QUANTITY"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "AMOUNT",
      "CHANNEL",
      "QUANTITY"
    FROM "menura_2_filter"
    WHERE "QUANTITY" = {p1:Int64}
  ),
  "menura_4_aggregate" AS (
    SELECT
      "CHANNEL" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_1_component_2"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_1_component_2" AS "metric_1_component_2"
    FROM "menura_4_aggregate"
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "channel",
      CAST(CASE WHEN toFloat64("metric_1_component_2") = 0 THEN NULL ELSE toFloat64("metric_1_component_1") / toFloat64("metric_1_component_2") END AS Nullable(Float64)) AS "per_unit"
    FROM "menura_5_project"
  )
SELECT
  "channel",
  "per_unit"
FROM "menura_6_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1