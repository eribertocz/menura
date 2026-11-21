WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "quantity" AS "UNITS"
    FROM "main"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "quantity" AS "BILLING.QUANTITY",
      "Sales Region" AS "BILLING.REGION"
    FROM (select * from "raw"."orders"
    )
  ),
  "menura_3_join" AS (
    SELECT
      "QUANTITY",
      "UNITS",
      "BILLING.QUANTITY",
      "BILLING.REGION"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."QUANTITY" = "menura_2_scan"."BILLING.QUANTITY"
  ),
  "menura_4_scan" AS (
    SELECT
      "quantity" AS "SHIPPING.UNITS",
      "Sales Region" AS "SHIPPING.REGION"
    FROM (select * from "raw"."orders"
    )
  ),
  "menura_5_join" AS (
    SELECT
      "QUANTITY",
      "UNITS",
      "BILLING.QUANTITY",
      "BILLING.REGION",
      "SHIPPING.UNITS",
      "SHIPPING.REGION"
    FROM "menura_3_join"
    LEFT JOIN "menura_4_scan" ON "menura_3_join"."UNITS" = "menura_4_scan"."SHIPPING.UNITS"
  )
SELECT
  "QUANTITY",
  "UNITS",
  "BILLING.QUANTITY",
  "BILLING.REGION",
  "SHIPPING.UNITS",
  "SHIPPING.REGION"
FROM "menura_5_join"