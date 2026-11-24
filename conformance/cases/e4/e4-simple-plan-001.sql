WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "region" AS "REGION",
      "sale_id" AS "SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "REGION",
      "SALE_ID"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_1_component_1",
      CAST(COUNT("SALE_ID") AS BIGINT) AS "metric_2_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "total_amount",
      "metric_2_component_1" AS "sale_count"
    FROM "menura_3_aggregate"
  )
SELECT
  "region",
  "total_amount",
  "sale_count"
FROM "menura_4_project"