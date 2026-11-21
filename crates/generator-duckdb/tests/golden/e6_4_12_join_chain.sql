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
  ),
  "menura_4_scan" AS (
    SELECT
      "Sales Region" AS "ORDERS_STAGED.STAGED_ORDERS.REGION",
      "quantity" AS "ORDERS_STAGED.STAGED_ORDERS.UNITS"
    FROM "main"."orders"
  ),
  "menura_5_join" AS (
    SELECT
      "QUANTITY",
      "REGION",
      "ORDERS_STAGED.QUANTITY",
      "ORDERS_STAGED.REGION",
      "ORDERS_STAGED.STAGED_ORDERS.REGION",
      "ORDERS_STAGED.STAGED_ORDERS.UNITS"
    FROM "menura_3_join"
    LEFT JOIN "menura_4_scan" ON "menura_3_join"."ORDERS_STAGED.REGION" = "menura_4_scan"."ORDERS_STAGED.STAGED_ORDERS.REGION"
  )
SELECT
  "QUANTITY",
  "REGION",
  "ORDERS_STAGED.QUANTITY",
  "ORDERS_STAGED.REGION",
  "ORDERS_STAGED.STAGED_ORDERS.REGION",
  "ORDERS_STAGED.STAGED_ORDERS.UNITS"
FROM "menura_5_join"