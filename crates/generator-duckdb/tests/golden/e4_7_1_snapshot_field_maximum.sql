WITH
  "menura_1_scan" AS (
    SELECT
      "warehouse" AS "WAREHOUSE",
      "product_id" AS "PRODUCT_ID",
      "category" AS "CATEGORY",
      "quantity" AS "QUANTITY"
    FROM "main"."stock"
  ),
  "menura_2_values" AS (
    SELECT
      "CATEGORY" AS "CATEGORY",
      "PRODUCT_ID" AS "PRODUCT_ID",
      "WAREHOUSE" AS "menura.over",
      SUM("QUANTITY") AS "QUANTITY"
    FROM "menura_1_scan"
    GROUP BY 1, 2, 3
  ),
  "menura_3_snapshot" AS (
    SELECT
      "CATEGORY",
      "PRODUCT_ID",
      CAST(MAX("QUANTITY") AS BIGINT) AS "QUANTITY"
    FROM "menura_2_values"
    GROUP BY 1, 2
  )
SELECT
  "CATEGORY",
  "PRODUCT_ID",
  "QUANTITY"
FROM "menura_3_snapshot"