WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "QUANTITY",
      "REGION"
    FROM "menura_1_scan"
    WHERE "REGION" IN (SELECT unnest(CAST($1 AS VARCHAR[])))
  ),
  "menura_3_filter" AS (
    SELECT
      "QUANTITY",
      "REGION"
    FROM "menura_2_filter"
    WHERE "QUANTITY" NOT IN (SELECT unnest(CAST($2 AS BIGINT[])))
  )
SELECT
  "QUANTITY",
  "REGION"
FROM "menura_3_filter"