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
      "customer_id" AS "CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "store_id" AS "STORE_ID"
    FROM "shop"."customers"
  ),
  "menura_4_scan" AS (
    SELECT
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_5_scan" AS (
    SELECT
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_6_filter" AS (
    SELECT
      "STORE_ID"
    FROM "menura_5_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_7_related" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_4_scan"
    GROUP BY 1
  ),
  "menura_8_visible" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_6_filter"
    GROUP BY 1
  ),
  "menura_9_semijoin" AS (
    SELECT
      "menura_row"."CUSTOMER_ID",
      "menura_row"."STORE_ID"
    FROM "menura_3_scan" AS "menura_row"
    JOIN "menura_7_related" AS "menura_related" ON "menura_row"."STORE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_8_visible" AS "menura_visible" ON "menura_row"."STORE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_10_related" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_11_visible" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_9_semijoin"
    GROUP BY 1
  ),
  "menura_12_semijoin" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."CUSTOMER_ID",
      "menura_row"."menura.row_identity"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_10_related" AS "menura_related" ON "menura_row"."CUSTOMER_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_11_visible" AS "menura_visible" ON "menura_row"."CUSTOMER_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_13_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "name" AS "ORDERS_CUSTOMERS.NAME",
      "store_id" AS "ORDERS_CUSTOMERS.STORE_ID"
    FROM "shop"."customers"
  ),
  "menura_14_scan" AS (
    SELECT
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_15_scan" AS (
    SELECT
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_16_filter" AS (
    SELECT
      "STORE_ID"
    FROM "menura_15_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_17_related" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_14_scan"
    GROUP BY 1
  ),
  "menura_18_visible" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_16_filter"
    GROUP BY 1
  ),
  "menura_19_semijoin" AS (
    SELECT
      "menura_row"."ORDERS_CUSTOMERS.CUSTOMER_ID",
      "menura_row"."ORDERS_CUSTOMERS.NAME",
      "menura_row"."ORDERS_CUSTOMERS.STORE_ID"
    FROM "menura_13_scan" AS "menura_row"
    JOIN "menura_17_related" AS "menura_related" ON "menura_row"."ORDERS_CUSTOMERS.STORE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_18_visible" AS "menura_visible" ON "menura_row"."ORDERS_CUSTOMERS.STORE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_20_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.NAME",
      "ORDERS_CUSTOMERS.STORE_ID"
    FROM "menura_12_semijoin"
    LEFT JOIN "menura_19_semijoin" ON "menura_12_semijoin"."CUSTOMER_ID" = "menura_19_semijoin"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_21_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.NAME",
      "ORDERS_CUSTOMERS.STORE_ID"
    FROM "menura_20_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.NAME", "menura.row_identity") = 1
  ),
  "menura_22_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.NAME" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_21_deduplicate"
    GROUP BY 1
  ),
  "menura_23_project" AS (
    SELECT
      "dimension_1" AS "name",
      "metric_1_component_1" AS "order_total"
    FROM "menura_22_aggregate"
  )
SELECT
  "name",
  "order_total"
FROM "menura_23_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1