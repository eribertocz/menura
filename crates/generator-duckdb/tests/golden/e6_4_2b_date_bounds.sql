WITH
  "menura_1_scan" AS (
    SELECT
      "created_at" AS "CREATED_AT",
      "ordered_on" AS "ORDERED_ON",
      "shipped_at" AS "SHIPPED_AT"
    FROM "main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "CREATED_AT",
      "ORDERED_ON",
      "SHIPPED_AT"
    FROM "menura_1_scan"
    WHERE ((("CREATED_AT" >= CAST(CAST($1 AS DATE) AS TIMESTAMP) AT TIME ZONE CAST($2 AS VARCHAR)) AND (CAST(CAST($3 AS DATE) AS TIMESTAMP) AT TIME ZONE CAST($2 AS VARCHAR) > "CREATED_AT")) AND ("SHIPPED_AT" BETWEEN CAST(CAST($4 AS DATE) AS TIMESTAMP) AND CAST(CAST($5 AS DATE) AS TIMESTAMP))) AND ("ORDERED_ON" < CAST($6 AS DATE))
  )
SELECT
  "CREATED_AT",
  "ORDERED_ON",
  "SHIPPED_AT"
FROM "menura_2_filter"