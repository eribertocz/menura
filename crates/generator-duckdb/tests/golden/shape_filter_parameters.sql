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
    WHERE ("REGION" = CAST($1 AS VARCHAR)) AND ("AMOUNT" > CAST($2 AS DECIMAL(4,2)))
  )
SELECT
  "AMOUNT",
  "REGION"
FROM "menura_2_filter"