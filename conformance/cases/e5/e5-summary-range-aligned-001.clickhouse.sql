WITH
  "menura_1_scan" AS (
    SELECT
      "amount_sum" AS "AMOUNT_SUM",
      "order_month" AS "ORDER_MONTH",
      "region" AS "REGION"
    FROM "shop"."orders_by_region_month"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH",
      "REGION"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH",
      "REGION"
    FROM "menura_2_filter"
    WHERE ("ORDER_MONTH" >= {p1:Date32}) AND ("ORDER_MONTH" <= {p2:Date32})
  ),
  "menura_4_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT_SUM")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_4_aggregate"
  )
SELECT
  "region",
  "total_amount"
FROM "menura_5_project"
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