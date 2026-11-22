WITH
  "menura_1_scan" AS (
    SELECT
      "order_date" AS "ORDER_DATE",
      "price" AS "PRICE"
    FROM "shop"."main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "ORDER_DATE") AS DATE) AS "dimension_1",
      CAST(SUM("PRICE") AS DOUBLE) AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "total_price"
    FROM "menura_2_aggregate"
  )
SELECT
  "order_date",
  "total_price"
FROM "menura_3_project"