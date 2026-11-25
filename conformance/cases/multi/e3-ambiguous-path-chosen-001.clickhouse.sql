WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_day" AS "ORDER_DAY",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "day" AS "ORDER_DATE.DAY",
      "month" AS "ORDER_DATE.MONTH"
    FROM "shop"."calendar"
  ),
  "menura_3_join" AS (
    SELECT
      "AMOUNT",
      "ORDER_DAY",
      "menura.row_identity",
      "ORDER_DATE.DAY",
      "ORDER_DATE.MONTH"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."ORDER_DAY" = "menura_2_scan"."ORDER_DATE.DAY"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "AMOUNT",
      "ORDER_DAY",
      "menura.row_identity",
      "ORDER_DATE.DAY",
      "ORDER_DATE.MONTH"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDER_DATE.MONTH", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "ORDER_DATE.MONTH" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "month",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_5_aggregate"
  )
SELECT
  "month",
  "total_amount"
FROM "menura_6_project"
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