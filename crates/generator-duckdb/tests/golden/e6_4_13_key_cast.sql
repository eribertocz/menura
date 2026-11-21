WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY"
    FROM "main"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "Amount" AS "ORDERS_STAGED.AMOUNT",
      "Sales Region" AS "ORDERS_STAGED.REGION"
    FROM (select * from "raw"."orders"
    )
  ),
  "menura_3_join" AS (
    SELECT
      "QUANTITY",
      "ORDERS_STAGED.AMOUNT",
      "ORDERS_STAGED.REGION"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON CAST("menura_1_scan"."QUANTITY" AS DECIMAL(38,2)) = CAST("menura_2_scan"."ORDERS_STAGED.AMOUNT" AS DECIMAL(38,2))
  )
SELECT
  "QUANTITY",
  "ORDERS_STAGED.AMOUNT",
  "ORDERS_STAGED.REGION"
FROM "menura_3_join"