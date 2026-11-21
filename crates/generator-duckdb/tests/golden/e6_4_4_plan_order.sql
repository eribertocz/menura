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
    WHERE "REGION" <> CAST($1 AS VARCHAR)
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "TOTAL_AMOUNT"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "REGION" AS "REGION",
      "ORDER_COUNT" AS "ORDER_COUNT",
      "TOTAL_AMOUNT" AS "TOTAL_AMOUNT"
    FROM "menura_3_aggregate"
  ),
  "menura_5_sort" AS (
    SELECT
      "REGION",
      "ORDER_COUNT",
      "TOTAL_AMOUNT"
    FROM "menura_4_project"
    ORDER BY "TOTAL_AMOUNT" DESC NULLS LAST
  ),
  "menura_6_limit" AS (
    SELECT
      "REGION",
      "ORDER_COUNT",
      "TOTAL_AMOUNT"
    FROM "menura_5_sort"
    ORDER BY "TOTAL_AMOUNT" DESC NULLS LAST
    LIMIT 10
  )
SELECT
  "REGION",
  "ORDER_COUNT",
  "TOTAL_AMOUNT"
FROM "menura_6_limit"
ORDER BY "TOTAL_AMOUNT" DESC NULLS LAST