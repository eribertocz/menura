WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "ship_date" AS "SHIP_DATE",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "date_key" AS "ORDERS_SHIP_DATE.DATE_KEY"
    FROM "shop"."calendar"
  ),
  "menura_3_join" AS (
    SELECT
      "AMOUNT",
      "SHIP_DATE",
      "menura.row_identity",
      "ORDERS_SHIP_DATE.DATE_KEY"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."SHIP_DATE" = "menura_2_scan"."ORDERS_SHIP_DATE.DATE_KEY"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "AMOUNT",
      "SHIP_DATE",
      "menura.row_identity",
      "ORDERS_SHIP_DATE.DATE_KEY"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_SHIP_DATE.DATE_KEY", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "ORDERS_SHIP_DATE.DATE_KEY" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "date_key",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_5_aggregate"
  )
SELECT
  "date_key",
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
  decimal_check_overflow = 1