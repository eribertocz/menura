WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "order_id" AS "ORDER_ID"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "ORDER_ID"
    FROM "menura_1_scan"
    WHERE FALSE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("ORDER_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_2_component_1"
    FROM "menura_2_filter"
  ),
  "menura_4_project" AS (
    SELECT
      "metric_1_component_1" AS "order_count",
      "metric_2_component_1" AS "total_amount"
    FROM "menura_3_aggregate"
  )
SELECT
  "order_count",
  "total_amount"
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