WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "ordered_on" AS "ORDERED_ON",
      "order_id" AS "ORDER_ID",
      "status" AS "STATUS"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "STATUS" AS "dimension_1",
      accurateCast(sumOrNull(toInt256(accurateCast(sign(CAST(sign((CASE WHEN toFloat64(toInt64(2)) = 0 THEN NULL ELSE toFloat64("AMOUNT") / toFloat64(toInt64(2)) END)) * floor(abs((CASE WHEN toFloat64(toInt64(2)) = 0 THEN NULL ELSE toFloat64("AMOUNT") / toFloat64(toInt64(2)) END)) + 0.5) AS Nullable(Float64))) * floor(abs(CAST(sign((CASE WHEN toFloat64(toInt64(2)) = 0 THEN NULL ELSE toFloat64("AMOUNT") / toFloat64(toInt64(2)) END)) * floor(abs((CASE WHEN toFloat64(toInt64(2)) = 0 THEN NULL ELSE toFloat64("AMOUNT") / toFloat64(toInt64(2)) END)) + 0.5) AS Nullable(Float64))) + 0.5), 'Nullable(Int32)'))), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(coalesce(count(if(ifNull((accurateCast(toDayOfMonth("ORDERED_ON"), 'Nullable(Int64)') >= toInt64(2)) AND (CAST(addDays("ORDERED_ON", toInt64(toInt64(1))) AS Nullable(Date32)) <= toDate32('2026-11-04')), 0), "ORDER_ID", NULL)), 0), 'Nullable(Int64)') AS "metric_2_component_1",
      accurateCast(coalesce(count(if(ifNull(CAST(startsWith(CAST(lowerUTF8("STATUS") AS Nullable(String)), 'op') AS Nullable(Bool)), 0), "ORDER_ID", NULL)), 0), 'Nullable(Int64)') AS "metric_3_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "status",
      "metric_1_component_1" AS "rounded_halves",
      "metric_2_component_1" AS "late_orders",
      "metric_3_component_1" AS "open_prefix_orders"
    FROM "menura_2_aggregate"
  )
SELECT
  "status",
  "rounded_halves",
  "late_orders",
  "open_prefix_orders"
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