WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      "AMOUNT" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "amount",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_3_aggregate"
  )
SELECT
  "amount",
  "total_amount"
FROM "menura_4_project"