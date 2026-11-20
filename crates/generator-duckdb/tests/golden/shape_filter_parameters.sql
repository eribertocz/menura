WITH
  "menura_1_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "REGION"
    FROM "menura_1_scan"
    WHERE ("REGION" = $1) AND ("AMOUNT" > $2)
  )
SELECT
  "AMOUNT",
  "REGION"
FROM "menura_2_filter"