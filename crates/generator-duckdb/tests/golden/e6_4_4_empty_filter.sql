WITH
  "menura_1_scan" AS (
    SELECT
      TRUE AS "menura.source.."
    FROM "main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      TRUE AS "menura.row"
    FROM "menura_1_scan"
    WHERE CAST($1 AS BOOLEAN)
  ),
  "menura_3_aggregate" AS (
    SELECT
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT"
    FROM "menura_2_filter"
  ),
  "menura_4_project" AS (
    SELECT
      "ORDER_COUNT" AS "ORDER_COUNT"
    FROM "menura_3_aggregate"
  )
SELECT
  "ORDER_COUNT"
FROM "menura_4_project"