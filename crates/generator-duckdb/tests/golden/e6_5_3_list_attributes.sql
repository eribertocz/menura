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
    WHERE "REGION" IN (SELECT unnest($1))
  ),
  "menura_3_filter" AS (
    SELECT
      "QUANTITY",
      "REGION"
    FROM "menura_2_filter"
    WHERE "QUANTITY" NOT IN (SELECT unnest($2))
  )
SELECT
  "QUANTITY",
  "REGION"
FROM "menura_3_filter"