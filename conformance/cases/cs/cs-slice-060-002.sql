WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "ORDER_DATE") AS DATE) AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_3_aggregate"
  )
SELECT
  "order_date",
  "total_amount"
FROM "menura_4_project"