WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY"
    FROM "main"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      CAST("quantity" AS HUGEINT) * CAST("quantity" AS HUGEINT) AS "ORDERS_STAGED.LINE_SQUARE",
      "Sales Region" AS "ORDERS_STAGED.REGION"
    FROM (select * from "raw"."orders"
    )
  ),
  "menura_3_join" AS (
    SELECT
      "QUANTITY",
      "ORDERS_STAGED.LINE_SQUARE",
      "ORDERS_STAGED.REGION"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON CAST("menura_1_scan"."QUANTITY" AS HUGEINT) = CAST("menura_2_scan"."ORDERS_STAGED.LINE_SQUARE" AS HUGEINT)
  )
SELECT
  "QUANTITY",
  "ORDERS_STAGED.LINE_SQUARE",
  "ORDERS_STAGED.REGION"
FROM "menura_3_join"