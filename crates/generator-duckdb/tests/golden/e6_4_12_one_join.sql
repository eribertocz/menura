WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "quantity" AS "ORDERS_STAGED.QUANTITY",
      "Sales Region" AS "ORDERS_STAGED.REGION"
    FROM (select * from "raw"."orders"
    )
  ),
  "menura_3_join" AS (
    SELECT
      "QUANTITY",
      "REGION",
      "ORDERS_STAGED.QUANTITY",
      "ORDERS_STAGED.REGION"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."QUANTITY" = "menura_2_scan"."ORDERS_STAGED.QUANTITY"
  )
SELECT
  "QUANTITY",
  "REGION",
  "ORDERS_STAGED.QUANTITY",
  "ORDERS_STAGED.REGION"
FROM "menura_3_join"