WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION",
      "ordered_on" AS "ORDERED_ON"
    FROM "main"."orders"
  ),
  "menura_2_deduplicate" AS (
    SELECT
      "QUANTITY",
      "REGION",
      "ORDERED_ON"
    FROM "menura_1_scan"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "REGION", "ORDERED_ON", "QUANTITY") = 1
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT"
    FROM "menura_2_deduplicate"
    GROUP BY 1
  )
SELECT
  "REGION",
  "ORDER_COUNT"
FROM "menura_3_aggregate"