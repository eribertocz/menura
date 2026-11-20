WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY"
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(SUM("QUANTITY") AS HUGEINT) AS "TOTAL_QUANTITY",
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT"
    FROM "menura_1_scan"
  ),
  "menura_3_project" AS (
    SELECT
      CAST(CASE WHEN CAST("ORDER_COUNT" AS DOUBLE) = 0 THEN NULL ELSE CAST("TOTAL_QUANTITY" AS DOUBLE) / CAST("ORDER_COUNT" AS DOUBLE) END AS DOUBLE) AS "AVERAGE_QUANTITY"
    FROM "menura_2_aggregate"
  )
SELECT
  "AVERAGE_QUANTITY"
FROM "menura_3_project"