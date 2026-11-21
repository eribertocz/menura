WITH
  "menura_1_scan" AS (
    SELECT
      "Amount" AS "AMOUNT_EXACT"
    FROM "shop"."main"."payments"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(SUM("AMOUNT_EXACT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_1_scan"
  ),
  "menura_3_project" AS (
    SELECT
      "metric_1_component_1" AS "total_exact"
    FROM "menura_2_aggregate"
  )
SELECT
  "total_exact"
FROM "menura_3_project"