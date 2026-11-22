WITH
  "menura_1_scan" AS (
    SELECT
      "amount_sum" AS "AMOUNT_SUM",
      "order_month" AS "ORDER_MONTH",
      "region" AS "REGION"
    FROM "shop"."main"."orders_by_region_month"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH",
      "REGION"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH",
      "REGION"
    FROM "menura_2_filter"
    WHERE ("ORDER_MONTH" >= CAST($1 AS DATE)) AND ("ORDER_MONTH" <= CAST($2 AS DATE))
  ),
  "menura_4_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      CAST(SUM("AMOUNT_SUM") AS HUGEINT) AS "metric_1_component_1"
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