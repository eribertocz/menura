WITH
  "menura_1_scan" AS (
    SELECT
      "order_id" AS "ORDER_ID",
      "customer_id" AS "CUSTOMER_ID"
    FROM "main"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "country_id" AS "COUNTRY_ID",
      "Sales Region" AS "REGION"
    FROM "main"."customers"
  ),
  "menura_3_related" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows",
      COUNT(*) FILTER (WHERE "REGION" = 'north') AS "menura.visible_rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_4_semijoin" AS (
    SELECT
      "menura_row"."ORDER_ID",
      "menura_row"."CUSTOMER_ID"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_3_related" AS "menura_related" ON "menura_row"."CUSTOMER_ID" = "menura_related"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_related"."menura.visible_rows"
  )
SELECT
  "ORDER_ID",
  "CUSTOMER_ID"
FROM "menura_4_semijoin"