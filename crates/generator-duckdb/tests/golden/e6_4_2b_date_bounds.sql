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
    WHERE ((("CREATED_AT" >= CAST($1 AS TIMESTAMP) AT TIME ZONE $2) AND (CAST($3 AS TIMESTAMP) AT TIME ZONE $2 > "CREATED_AT")) AND ("SHIPPED_AT" BETWEEN CAST($4 AS TIMESTAMP) AND CAST($5 AS TIMESTAMP))) AND ("ORDERED_ON" < $6)
  )
SELECT
  "CREATED_AT",
  "ORDERED_ON",
  "SHIPPED_AT"
FROM "menura_2_filter"