WITH
  "menura_1_scan" AS (
    SELECT
      "region" AS "REGION",
      "quantity_count" AS "QUANTITY_COUNT"
    FROM "main"."orders_by_region"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(COALESCE(SUM("QUANTITY_COUNT"), 0) AS BIGINT) AS "QUANTITY_COUNT"
    FROM "menura_1_scan"
    GROUP BY 1
  )
SELECT
  "REGION",
  "QUANTITY_COUNT"
FROM "menura_2_aggregate"