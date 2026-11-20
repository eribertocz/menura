WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY"
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      -CAST("QUANTITY" AS HUGEINT) AS "NEGATED",
      CAST(SUM((CAST("QUANTITY" AS HUGEINT) * CAST("QUANTITY" AS HUGEINT)) + CAST("QUANTITY" AS HUGEINT)) AS HUGEINT) AS "SQUARES"
    FROM "menura_1_scan"
    GROUP BY 1
  )
SELECT
  "NEGATED",
  "SQUARES"
FROM "menura_2_aggregate"