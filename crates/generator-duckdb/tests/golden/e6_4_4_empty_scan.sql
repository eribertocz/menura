WITH
  "menura_1_scan" AS (
    SELECT
      TRUE AS "menura.source."
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT"
    FROM "menura_1_scan"
  ),
  "menura_3_project" AS (
    SELECT
      "ORDER_COUNT" AS "ORDER_COUNT"
    FROM "menura_2_aggregate"
  )
SELECT
  "ORDER_COUNT"
FROM "menura_3_project"