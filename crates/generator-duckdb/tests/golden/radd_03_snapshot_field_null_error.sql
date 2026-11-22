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
      CASE WHEN SUM("QUANTITY") IS NULL THEN error('RADD-03: nulls: error and a NULL value exists in metric STOCK_LEVEL') ELSE SUM("QUANTITY") END AS "QUANTITY"
    FROM "menura_1_scan"
    GROUP BY 1, 2, 3
  ),
  "menura_3_snapshot" AS (
    SELECT
      "CATEGORY",
      "PRODUCT_ID",
      CAST(MIN("QUANTITY") AS BIGINT) AS "QUANTITY"
    FROM "menura_2_values"
    GROUP BY 1, 2
  )
SELECT
  "CATEGORY",
  "PRODUCT_ID",
  "QUANTITY"
FROM "menura_3_snapshot"