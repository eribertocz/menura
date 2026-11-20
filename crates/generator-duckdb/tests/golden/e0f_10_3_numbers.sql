WITH
  "menura_1_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "created_at" AS "CREATED_AT",
      "ordered_on" AS "ORDERED_ON",
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_project" AS (
    SELECT
      CAST(CAST(round("AMOUNT", 1) AS DECIMAL(10,1)) AS DECIMAL(10,1)) AS "ROUNDED",
      CAST(CAST(CASE WHEN $1 = 0 THEN NULL ELSE CAST("QUANTITY" AS HUGEINT) % CAST($1 AS HUGEINT) END AS HUGEINT) AS HUGEINT) AS "REMAINDER",
      CAST(CAST(coalesce(CAST("QUANTITY" AS BIGINT), CAST($2 AS BIGINT)) AS BIGINT) AS BIGINT) AS "FILLED"
    FROM "menura_1_scan"
  )
SELECT
  "ROUNDED",
  "REMAINDER",
  "FILLED"
FROM "menura_2_project"