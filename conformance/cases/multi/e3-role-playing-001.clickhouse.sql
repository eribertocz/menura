WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_day" AS "ORDER_DAY",
      "ship_day" AS "SHIP_DAY",
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
      "SHIP_DAY",
      "menura.row_identity",
      "ORDER_DATE.DAY",
      "ORDER_DATE.MONTH"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."ORDER_DAY" = "menura_2_scan"."ORDER_DATE.DAY"
  ),
  "menura_4_scan" AS (
    SELECT
      "day" AS "SHIP_DATE.DAY",
      "fiscal_year" AS "SHIP_DATE.FISCAL_YEAR"
    FROM "shop"."calendar"
  ),
  "menura_5_join" AS (
    SELECT
      "AMOUNT",
      "ORDER_DAY",
      "SHIP_DAY",
      "menura.row_identity",
      "ORDER_DATE.DAY",
      "ORDER_DATE.MONTH",
      "SHIP_DATE.DAY",
      "SHIP_DATE.FISCAL_YEAR"
    FROM "menura_3_join"
    LEFT JOIN "menura_4_scan" ON "menura_3_join"."SHIP_DAY" = "menura_4_scan"."SHIP_DATE.DAY"
  ),
  "menura_6_scan" AS (
    SELECT
      "fiscal_year" AS "SHIP_DATE.CALENDAR_FISCAL_YEAR.FISCAL_YEAR",
      "label" AS "SHIP_DATE.CALENDAR_FISCAL_YEAR.LABEL"
    FROM "shop"."fiscal_years"
  ),
  "menura_7_join" AS (
    SELECT
      "AMOUNT",
      "ORDER_DAY",
      "SHIP_DAY",
      "menura.row_identity",
      "ORDER_DATE.DAY",
      "ORDER_DATE.MONTH",
      "SHIP_DATE.DAY",
      "SHIP_DATE.FISCAL_YEAR",
      "SHIP_DATE.CALENDAR_FISCAL_YEAR.FISCAL_YEAR",
      "SHIP_DATE.CALENDAR_FISCAL_YEAR.LABEL"
    FROM "menura_5_join"
    LEFT JOIN "menura_6_scan" ON "menura_5_join"."SHIP_DATE.FISCAL_YEAR" = "menura_6_scan"."SHIP_DATE.CALENDAR_FISCAL_YEAR.FISCAL_YEAR"
  ),
  "menura_8_deduplicate" AS (
    SELECT
      "AMOUNT",
      "ORDER_DAY",
      "SHIP_DAY",
      "menura.row_identity",
      "ORDER_DATE.DAY",
      "ORDER_DATE.MONTH",
      "SHIP_DATE.DAY",
      "SHIP_DATE.FISCAL_YEAR",
      "SHIP_DATE.CALENDAR_FISCAL_YEAR.FISCAL_YEAR",
      "SHIP_DATE.CALENDAR_FISCAL_YEAR.LABEL"
    FROM "menura_7_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDER_DATE.MONTH", "SHIP_DATE.CALENDAR_FISCAL_YEAR.LABEL", "menura.row_identity") = 1
  ),
  "menura_9_aggregate" AS (
    SELECT
      "ORDER_DATE.MONTH" AS "dimension_1",
      "SHIP_DATE.CALENDAR_FISCAL_YEAR.LABEL" AS "dimension_2",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_8_deduplicate"
    GROUP BY 1, 2
  ),
  "menura_10_project" AS (
    SELECT
      "dimension_1" AS "month",
      "dimension_2" AS "label",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_9_aggregate"
  )
SELECT
  "month",
  "label",
  "total_amount"
FROM "menura_10_project"
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