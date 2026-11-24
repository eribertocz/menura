WITH
  "menura_1_scan" AS (
    SELECT
      "order_count" AS "ORDER_COUNT",
      "order_month" AS "ORDER_MONTH"
    FROM "shop"."orders_price_by_month"
  ),
  "menura_2_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_MONTH")) AS "dimension_1",
      accurateCast(coalesce(sum(toInt256("ORDER_COUNT")), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "order_count"
    FROM "menura_2_aggregate"
  )
SELECT
  "order_date",
  "order_count"
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