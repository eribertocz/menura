WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "created_at" AS "CREATED_AT",
      "ordered_on" AS "ORDERED_ON"
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
      "CREATED_AT",
      "ORDERED_ON",
      "ORDERS_STAGED.QUANTITY",
      "ORDERS_STAGED.REGION"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."QUANTITY" = "menura_2_scan"."ORDERS_STAGED.QUANTITY"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "QUANTITY",
      "CREATED_AT",
      "ORDERED_ON",
      "ORDERS_STAGED.QUANTITY",
      "ORDERS_STAGED.REGION"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY date_trunc('month', "CREATED_AT" AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR), "ORDERS_STAGED.REGION", "ORDERED_ON") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      date_trunc('month', "CREATED_AT" AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR) AS "CREATED_MONTH",
      "ORDERS_STAGED.REGION" AS "ORDERS_STAGED.REGION",
      CAST(SUM("QUANTITY") AS DECIMAL(38,0)) AS "TOTAL_QUANTITY"
    FROM "menura_4_deduplicate"
    GROUP BY 1, 2
  )
SELECT
  "CREATED_MONTH",
  "ORDERS_STAGED.REGION",
  "TOTAL_QUANTITY"
FROM "menura_5_aggregate"