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
      "country_id" AS "COUNTRY_ID"
    FROM "main"."customers"
  ),
  "menura_4_scan" AS (
    SELECT
      "country_id" AS "COUNTRY_ID"
    FROM "main"."countries"
  ),
  "menura_5_scan" AS (
    SELECT
      "country_id" AS "COUNTRY_ID",
      "is_open" AS "IS_OPEN"
    FROM "main"."countries"
  ),
  "menura_6_filter" AS (
    SELECT
      "COUNTRY_ID",
      "IS_OPEN"
    FROM "menura_5_scan"
    WHERE "IS_OPEN"
  ),
  "menura_7_related" AS (
    SELECT
      "COUNTRY_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_4_scan"
    GROUP BY 1
  ),
  "menura_8_visible" AS (
    SELECT
      "COUNTRY_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_6_filter"
    GROUP BY 1
  ),
  "menura_9_semijoin" AS (
    SELECT
      "menura_row"."CUSTOMER_ID",
      "menura_row"."COUNTRY_ID"
    FROM "menura_3_scan" AS "menura_row"
    JOIN "menura_7_related" AS "menura_related" ON "menura_row"."COUNTRY_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_8_visible" AS "menura_visible" ON "menura_row"."COUNTRY_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_10_related" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_11_visible" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_9_semijoin"
    GROUP BY 1
  ),
  "menura_12_semijoin" AS (
    SELECT
      "menura_row"."ORDER_ID",
      "menura_row"."CUSTOMER_ID"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_10_related" AS "menura_related" ON "menura_row"."CUSTOMER_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_11_visible" AS "menura_visible" ON "menura_row"."CUSTOMER_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  )
SELECT
  "ORDER_ID",
  "CUSTOMER_ID"
FROM "menura_12_semijoin"