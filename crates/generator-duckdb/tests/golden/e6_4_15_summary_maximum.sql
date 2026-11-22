WITH
  "menura_1_scan" AS (
    SELECT
      "region" AS "REGION",
      "amount_max" AS "AMOUNT_MAX"
    FROM "main"."orders_by_region"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(MAX("AMOUNT_MAX") AS DECIMAL(10,2)) AS "LARGEST_AMOUNT"
    FROM "menura_1_scan"
    GROUP BY 1
  )
SELECT
  "REGION",
  "LARGEST_AMOUNT"
FROM "menura_2_aggregate"