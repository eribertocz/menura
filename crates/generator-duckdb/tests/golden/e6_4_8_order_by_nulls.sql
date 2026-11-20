WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_sort" AS (
    SELECT
      "QUANTITY",
      "REGION"
    FROM "menura_1_scan"
    ORDER BY "REGION" ASC NULLS FIRST, "QUANTITY" DESC NULLS LAST
  ),
  "menura_3_limit" AS (
    SELECT
      "QUANTITY",
      "REGION"
    FROM "menura_2_sort"
    ORDER BY "REGION" ASC NULLS FIRST, "QUANTITY" DESC NULLS LAST
    LIMIT 5
  )
SELECT
  "QUANTITY",
  "REGION"
FROM "menura_3_limit"
ORDER BY "REGION" ASC NULLS FIRST, "QUANTITY" DESC NULLS LAST