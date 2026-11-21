WITH
  "menura_1_scan" AS (
    SELECT
      "order_id" AS "ORDER_ID",
      "customer_id" AS "CUSTOMER_ID"
    FROM "main"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID"
    FROM "main"."customers"
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "Sales Region" AS "REGION"
    FROM "main"."customers"
  ),
  "menura_4_filter" AS (
    SELECT
      "CUSTOMER_ID",
      "REGION"
    FROM "menura_3_scan"
    WHERE "REGION" = 'north'
  ),
  "menura_5_related" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_6_visible" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_4_filter"
    GROUP BY 1
  ),
  "menura_7_semijoin" AS (
    SELECT
      "menura_row"."ORDER_ID",
      "menura_row"."CUSTOMER_ID"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."CUSTOMER_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_6_visible" AS "menura_visible" ON "menura_row"."CUSTOMER_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  )
SELECT
  "ORDER_ID",
  "CUSTOMER_ID"
FROM "menura_7_semijoin"