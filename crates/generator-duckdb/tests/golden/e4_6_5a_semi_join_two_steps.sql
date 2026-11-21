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
  "menura_3_scan" AS (
    SELECT
      "country_id" AS "COUNTRY_ID",
      "is_open" AS "IS_OPEN"
    FROM "main"."countries"
  ),
  "menura_4_related" AS (
    SELECT
      "COUNTRY_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows",
      COUNT(*) FILTER (WHERE "IS_OPEN") AS "menura.visible_rows"
    FROM "menura_3_scan"
    GROUP BY 1
  ),
  "menura_5_related" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows",
      COUNT(*) FILTER (WHERE "menura_related"."menura.rows" = "menura_related"."menura.visible_rows") AS "menura.visible_rows"
    FROM "menura_2_scan" AS "menura_row"
    LEFT JOIN "menura_4_related" AS "menura_related" ON "menura_row"."COUNTRY_ID" = "menura_related"."menura.key_1"
    GROUP BY 1
  ),
  "menura_6_semijoin" AS (
    SELECT
      "menura_row"."ORDER_ID",
      "menura_row"."CUSTOMER_ID"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."CUSTOMER_ID" = "menura_related"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_related"."menura.visible_rows"
  )
SELECT
  "ORDER_ID",
  "CUSTOMER_ID"
FROM "menura_6_semijoin"