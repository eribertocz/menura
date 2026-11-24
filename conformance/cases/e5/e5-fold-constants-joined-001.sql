WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "order_id" AS "ORDER_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "ORDER_ID",
      "menura.row_identity"
    FROM "menura_1_scan"
    WHERE ("AMOUNT" >= 0) AND TRUE
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_4_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_5_filter" AS (
    SELECT
      "CUSTOMER_ID"
    FROM "menura_4_scan"
    WHERE FALSE AND TRUE
  ),
  "menura_6_related" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_3_scan"
    GROUP BY 1
  ),
  "menura_7_visible" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_5_filter"
    GROUP BY 1
  ),
  "menura_8_semijoin" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."CUSTOMER_ID",
      "menura_row"."ORDER_ID",
      "menura_row"."menura.row_identity"
    FROM "menura_2_filter" AS "menura_row"
    JOIN "menura_6_related" AS "menura_related" ON "menura_row"."CUSTOMER_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_7_visible" AS "menura_visible" ON "menura_row"."CUSTOMER_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_9_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "segment" AS "ORDERS_CUSTOMERS.SEGMENT"
    FROM "shop"."customers"
  ),
  "menura_10_filter" AS (
    SELECT
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT"
    FROM "menura_9_scan"
    WHERE FALSE AND TRUE
  ),
  "menura_11_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "ORDER_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT"
    FROM "menura_8_semijoin"
    LEFT JOIN "menura_10_filter" ON "menura_8_semijoin"."CUSTOMER_ID" = "menura_10_filter"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_12_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "ORDER_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT"
    FROM "menura_11_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.SEGMENT", "menura.row_identity") = 1
  ),
  "menura_13_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.SEGMENT" AS "dimension_1",
      CAST(COUNT("ORDER_ID") AS BIGINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_2_component_1"
    FROM "menura_12_deduplicate"
    GROUP BY 1
  ),
  "menura_14_project" AS (
    SELECT
      "dimension_1" AS "segment",
      "metric_1_component_1" AS "order_count",
      "metric_2_component_1" AS "total_amount"
    FROM "menura_13_aggregate"
  )
SELECT
  "segment",
  "order_count",
  "total_amount"
FROM "menura_14_project"