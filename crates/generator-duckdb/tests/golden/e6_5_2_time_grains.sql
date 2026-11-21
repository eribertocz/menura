WITH
  "menura_1_scan" AS (
    SELECT
      "created_at" AS "CREATED_AT",
      "ordered_on" AS "ORDERED_ON"
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "ORDERED_ON") AS DATE) AS "ORDERED_ON_MONTH",
      date_trunc('week', "CREATED_AT" AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR) AS "CREATED_AT_WEEK",
      date_trunc('day', "CREATED_AT" AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR) AS "CREATED_AT_DAY",
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT"
    FROM "menura_1_scan"
    GROUP BY 1, 2, 3
  )
SELECT
  "ORDERED_ON_MONTH",
  "CREATED_AT_WEEK",
  "CREATED_AT_DAY",
  "ORDER_COUNT"
FROM "menura_2_aggregate"