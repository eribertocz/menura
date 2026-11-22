WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "order_id" AS "ORDER_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "ORDER_ID",
      "menura.row_identity"
    FROM "menura_1_scan"
    WHERE FALSE AND TRUE
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "segment" AS "ORDERS_CUSTOMERS.SEGMENT"
    FROM "shop"."main"."customers"
  ),
  "menura_4_filter" AS (
    SELECT
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT"
    FROM "menura_3_scan"
    WHERE FALSE AND TRUE
  ),
  "menura_5_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "ORDER_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT"
    FROM "menura_2_filter"
    LEFT JOIN "menura_4_filter" ON "menura_2_filter"."CUSTOMER_ID" = "menura_4_filter"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_6_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.SEGMENT" AS "dimension_1",
      CAST(COUNT("ORDER_ID") AS BIGINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_2_component_1"
    FROM "menura_5_join"
    GROUP BY 1
  ),
  "menura_7_project" AS (
    SELECT
      "dimension_1" AS "segment",
      "metric_1_component_1" AS "order_count",
      "metric_2_component_1" AS "total_amount"
    FROM "menura_6_aggregate"
  )
SELECT
  "segment",
  "order_count",
  "total_amount"
FROM "menura_7_project"