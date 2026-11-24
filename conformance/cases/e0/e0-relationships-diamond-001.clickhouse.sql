WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "region_id" AS "ORDERS_CUSTOMERS.REGION_ID"
    FROM "shop"."customers"
  ),
  "menura_3_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_4_scan" AS (
    SELECT
      "region_id" AS "ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_ID",
      "name" AS "ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_NAME"
    FROM "shop"."regions"
  ),
  "menura_5_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION_ID",
      "ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_ID",
      "ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_NAME"
    FROM "menura_3_join"
    LEFT JOIN "menura_4_scan" ON "menura_3_join"."ORDERS_CUSTOMERS.REGION_ID" = "menura_4_scan"."ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_ID"
  ),
  "menura_6_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION_ID",
      "ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_ID",
      "ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_NAME"
    FROM "menura_5_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_NAME", "menura.row_identity") = 1
  ),
  "menura_7_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.CUSTOMERS_REGIONS.REGION_NAME" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_6_deduplicate"
    GROUP BY 1
  ),
  "menura_8_project" AS (
    SELECT
      "dimension_1" AS "region_name",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_7_aggregate"
  )
SELECT
  "region_name",
  "total_amount"
FROM "menura_8_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1