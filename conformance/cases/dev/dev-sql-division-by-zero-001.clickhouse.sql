WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "region" AS "REGION",
      "sale_id" AS "SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "REGION",
      "SALE_ID"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1",
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_2"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_1_component_2" AS "metric_1_component_2"
    FROM "menura_3_aggregate"
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "region",
      CAST(CASE WHEN toFloat64(accurateCast(toInt256("metric_1_component_2") - toInt256("metric_1_component_2"), 'Nullable(Int128)')) = 0 THEN NULL ELSE toFloat64("metric_1_component_1") / toFloat64(accurateCast(toInt256("metric_1_component_2") - toInt256("metric_1_component_2"), 'Nullable(Int128)')) END AS Nullable(Float64)) AS "per_nothing"
    FROM "menura_4_project"
  )
SELECT
  "region",
  "per_nothing"
FROM "menura_5_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1