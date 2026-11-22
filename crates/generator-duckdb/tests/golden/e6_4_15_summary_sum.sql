WITH
  "menura_1_scan" AS (
    SELECT
      "region" AS "REGION",
      "quantity_sum" AS "QUANTITY_SUM"
    FROM "main"."orders_by_region"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(SUM("QUANTITY_SUM") AS HUGEINT) AS "TOTAL_QUANTITY"
    FROM "menura_1_scan"
    GROUP BY 1
  )
SELECT
  "REGION",
  "TOTAL_QUANTITY"
FROM "menura_2_aggregate"