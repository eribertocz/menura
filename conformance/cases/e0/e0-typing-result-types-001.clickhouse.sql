WITH
  "menura_1_scan" AS (
    SELECT
      "discount" AS "DISCOUNT",
      "order_id" AS "ORDER_ID",
      "price" AS "PRICE",
      "quantity" AS "QUANTITY",
      "status" AS "STATUS"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "STATUS" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_2_component_1",
      accurateCast(coalesce(count("ORDER_ID"), 0), 'Nullable(Int64)') AS "metric_2_component_2",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_3_component_1",
      CAST(sumOrNull("PRICE") AS Nullable(Decimal(38, 2))) AS "metric_4_component_1",
      CAST(CASE WHEN coalesce(count("DISCOUNT"), 0) = 0 THEN NULL ELSE toFloat64(sum("DISCOUNT")) / toFloat64(count("DISCOUNT")) END AS Nullable(Float64)) AS "metric_5_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "status",
      "metric_1_component_1" AS "total_quantity",
      CAST(CASE WHEN toFloat64("metric_2_component_2") = 0 THEN NULL ELSE toFloat64("metric_2_component_1") / toFloat64("metric_2_component_2") END AS Nullable(Float64)) AS "quantity_per_order",
      CAST(CAST(coalesce(CAST(CASE WHEN toFloat64(toInt64(0)) = 0 THEN NULL ELSE toFloat64("metric_3_component_1") / toFloat64(toInt64(0)) END AS Nullable(Float64)), CAST(-1.0e0 AS Nullable(Float64))) AS Nullable(Float64)) AS Nullable(Float64)) AS "quantity_per_nothing",
      "metric_4_component_1" AS "total_price",
      "metric_5_component_1" AS "average_discount"
    FROM "menura_2_aggregate"
  )
SELECT
  "status",
  "total_quantity",
  "quantity_per_order",
  "quantity_per_nothing",
  "total_price",
  "average_discount"
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