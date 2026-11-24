WITH
  "menura_1_scan" AS (
    SELECT
      "channel" AS "CHANNEL",
      "quantity" AS "QUANTITY",
      "sale_date" AS "SALE_DATE"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "CHANNEL",
      "QUANTITY",
      "SALE_DATE"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "CHANNEL",
      "QUANTITY",
      "SALE_DATE"
    FROM "menura_2_filter"
    WHERE ("CHANNEL" = {p1:String}) AND (("SALE_DATE" >= {p2:Date32}) AND ("SALE_DATE" <= {p3:Date32}))
  ),
  "menura_4_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("SALE_DATE")) AS "dimension_1",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "sale_date",
      "metric_1_component_1" AS "total_quantity"
    FROM "menura_4_aggregate"
  ),
  "menura_6_sort" AS (
    SELECT
      "sale_date",
      "total_quantity"
    FROM "menura_5_project"
    ORDER BY "sale_date" DESC NULLS LAST
  ),
  "menura_7_limit" AS (
    SELECT
      "sale_date",
      "total_quantity"
    FROM "menura_6_sort"
    ORDER BY "sale_date" DESC NULLS LAST
    LIMIT 1
  )
SELECT
  "sale_date",
  "total_quantity"
FROM "menura_7_limit"
ORDER BY "sale_date" DESC NULLS LAST
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1