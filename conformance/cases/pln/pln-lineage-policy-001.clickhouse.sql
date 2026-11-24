WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "store_id" AS "STORE_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_3_scan" AS (
    SELECT
      "region" AS "REGION",
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_4_filter" AS (
    SELECT
      "REGION",
      "STORE_ID"
    FROM "menura_3_scan"
    WHERE ("REGION" = 'NORTH') AND TRUE
  ),
  "menura_5_related" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_6_visible" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_4_filter"
    GROUP BY 1
  ),
  "menura_7_semijoin" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."CUSTOMER_ID",
      "menura_row"."STORE_ID",
      "menura_row"."menura.row_identity"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."STORE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_6_visible" AS "menura_visible" ON "menura_row"."STORE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_8_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "segment" AS "ORDERS_CUSTOMERS.SEGMENT"
    FROM "shop"."customers"
  ),
  "menura_9_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STORE_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT"
    FROM "menura_7_semijoin"
    LEFT JOIN "menura_8_scan" ON "menura_7_semijoin"."CUSTOMER_ID" = "menura_8_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_10_scan" AS (
    SELECT
      "region" AS "ORDERS_STORES.REGION",
      "store_id" AS "ORDERS_STORES.STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_11_filter" AS (
    SELECT
      "ORDERS_STORES.REGION",
      "ORDERS_STORES.STORE_ID"
    FROM "menura_10_scan"
    WHERE ("ORDERS_STORES.REGION" = 'NORTH') AND TRUE
  ),
  "menura_12_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STORE_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT",
      "ORDERS_STORES.REGION",
      "ORDERS_STORES.STORE_ID"
    FROM "menura_9_join"
    LEFT JOIN "menura_11_filter" ON "menura_9_join"."STORE_ID" = "menura_11_filter"."ORDERS_STORES.STORE_ID"
  ),
  "menura_13_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STORE_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT",
      "ORDERS_STORES.REGION",
      "ORDERS_STORES.STORE_ID"
    FROM "menura_12_join"
    WHERE "ORDERS_CUSTOMERS.SEGMENT" = {p1:String}
  ),
  "menura_14_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STORE_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT",
      "ORDERS_STORES.REGION",
      "ORDERS_STORES.STORE_ID"
    FROM "menura_13_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_STORES.REGION", "menura.row_identity") = 1
  ),
  "menura_15_aggregate" AS (
    SELECT
      "ORDERS_STORES.REGION" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_14_deduplicate"
    GROUP BY 1
  ),
  "menura_16_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_15_aggregate"
  )
SELECT
  "region",
  "total_amount"
FROM "menura_16_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1