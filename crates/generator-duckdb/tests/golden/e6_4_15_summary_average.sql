WITH
  "menura_1_scan" AS (
    SELECT
      "region" AS "REGION",
      "amount_sum" AS "AMOUNT_SUM",
      "amount_count" AS "AMOUNT_COUNT"
    FROM "main"."orders_by_region"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(CASE WHEN COALESCE(SUM("AMOUNT_COUNT"), 0) = 0 THEN NULL ELSE CAST(SUM("AMOUNT_SUM") AS DOUBLE) / CAST(COALESCE(SUM("AMOUNT_COUNT"), 0) AS DOUBLE) END AS DOUBLE) AS "AVERAGE_AMOUNT"
    FROM "menura_1_scan"
    GROUP BY 1
  )
SELECT
  "REGION",
  "AVERAGE_AMOUNT"
FROM "menura_2_aggregate"