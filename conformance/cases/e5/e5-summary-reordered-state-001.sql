WITH
  "menura_1_scan" AS (
    SELECT
      "amount_count" AS "AMOUNT_COUNT",
      "amount_sum" AS "AMOUNT_SUM",
      "order_count" AS "ORDER_COUNT",
      "order_month" AS "ORDER_MONTH",
      "region" AS "REGION"
    FROM "shop"."orders_by_region_month"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT_COUNT",
      "AMOUNT_SUM",
      "ORDER_COUNT",
      "ORDER_MONTH",
      "REGION"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      CAST(date_trunc('month', "ORDER_MONTH") AS DATE) AS "dimension_2",
      CAST(SUM("AMOUNT_SUM") AS HUGEINT) AS "metric_1_component_1",
      CAST(COALESCE(SUM("ORDER_COUNT"), 0) AS BIGINT) AS "metric_2_component_1",
      CAST(CASE WHEN COALESCE(SUM("AMOUNT_COUNT"), 0) = 0 THEN NULL ELSE CAST(SUM("AMOUNT_SUM") AS DOUBLE) / CAST(COALESCE(SUM("AMOUNT_COUNT"), 0) AS DOUBLE) END AS DOUBLE) AS "metric_3_component_1"
    FROM "menura_2_filter"
    GROUP BY 1, 2
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "region",
      "dimension_2" AS "order_date",
      "metric_1_component_1" AS "total_amount",
      "metric_2_component_1" AS "order_count",
      "metric_3_component_1" AS "average_amount"
    FROM "menura_3_aggregate"
  )
SELECT
  "region",
  "order_date",
  "total_amount",
  "order_count",
  "average_amount"
FROM "menura_4_project"