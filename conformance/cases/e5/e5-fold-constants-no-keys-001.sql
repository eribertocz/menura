WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "order_id" AS "ORDER_ID"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "ORDER_ID"
    FROM "menura_1_scan"
    WHERE FALSE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      CAST(COUNT("ORDER_ID") AS BIGINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_2_component_1"
    FROM "menura_2_filter"
  ),
  "menura_4_project" AS (
    SELECT
      "metric_1_component_1" AS "order_count",
      "metric_2_component_1" AS "total_amount"
    FROM "menura_3_aggregate"
  )
SELECT
  "order_count",
  "total_amount"
FROM "menura_4_project"