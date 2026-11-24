WITH
  "menura_1_scan" AS (
    SELECT
      "amount_count" AS "AMOUNT_COUNT",
      "amount_sum" AS "AMOUNT_SUM",
      "order_count" AS "ORDER_COUNT",
      "order_month" AS "ORDER_MONTH",
      "region" AS "REGION"
    FROM "shop"."orders_by_region_month"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT_COUNT",
      "AMOUNT_SUM",
      "ORDER_COUNT",
      "ORDER_MONTH",
      "REGION"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      toDate32(toStartOfMonth("ORDER_MONTH")) AS "dimension_2",
      accurateCast(sumOrNull(toInt256("AMOUNT_SUM")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(coalesce(sum(toInt256("ORDER_COUNT")), 0), 'Nullable(Int64)') AS "metric_2_component_1",
      CAST(CASE WHEN coalesce(sum(toInt256("AMOUNT_COUNT")), 0) = 0 THEN NULL ELSE toFloat64(sum(toInt256("AMOUNT_SUM"))) / toFloat64(coalesce(sum(toInt256("AMOUNT_COUNT")), 0)) END AS Nullable(Float64)) AS "metric_3_component_1"
    FROM "menura_2_filter"
    GROUP BY 1, 2
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "region",
      "dimension_2" AS "order_date",
      "metric_1_component_1" AS "total_amount",
      "metric_2_component_1" AS "order_count",
      "metric_3_component_1" AS "average_amount"
    FROM "menura_3_aggregate"
  )
SELECT
  "region",
  "order_date",
  "total_amount",
  "order_count",
  "average_amount"
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