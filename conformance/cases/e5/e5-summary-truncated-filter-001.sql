WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE",
      "region" AS "REGION"
    FROM "shop"."main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE",
      "REGION"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE",
      "REGION"
    FROM "menura_2_filter"
    WHERE "ORDER_DATE" >= CAST($1 AS DATE)
  ),
  "menura_4_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_4_aggregate"
  )
SELECT
  "region",
  "total_amount"
FROM "menura_5_project"