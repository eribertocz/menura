WITH
  "menura_1_scan" AS (
    SELECT
      "order_count" AS "ORDER_COUNT",
      "order_month" AS "ORDER_MONTH"
    FROM "shop"."main"."orders_price_by_month"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "ORDER_MONTH") AS DATE) AS "dimension_1",
      CAST(COALESCE(SUM("ORDER_COUNT"), 0) AS BIGINT) AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "order_count"
    FROM "menura_2_aggregate"
  )
SELECT
  "order_date",
  "order_count"
FROM "menura_3_project"