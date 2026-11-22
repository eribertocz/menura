WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "store_id" AS "STORE_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."main"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "store_id" AS "STORE_ID"
    FROM "shop"."main"."stores"
  ),
  "menura_3_scan" AS (
    SELECT
      "region" AS "REGION",
      "store_id" AS "STORE_ID"
    FROM "shop"."main"."stores"
  ),
  "menura_4_filter" AS (
    SELECT
      "REGION",
      "STORE_ID"
    FROM "menura_3_scan"
    WHERE ("REGION" = 'NORTH') AND TRUE
  ),
  "menura_5_related" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_6_visible" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_4_filter"
    GROUP BY 1
  ),
  "menura_7_semijoin" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."STORE_ID",
      "menura_row"."menura.row_identity"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."STORE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_6_visible" AS "menura_visible" ON "menura_row"."STORE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_8_scan" AS (
    SELECT
      "region" AS "ORDERS_STORES.REGION",
      "store_id" AS "ORDERS_STORES.STORE_ID"
    FROM "shop"."main"."stores"
  ),
  "menura_9_filter" AS (
    SELECT
      "ORDERS_STORES.REGION",
      "ORDERS_STORES.STORE_ID"
    FROM "menura_8_scan"
    WHERE ("ORDERS_STORES.REGION" = 'NORTH') AND TRUE
  ),
  "menura_10_join" AS (
    SELECT
      "AMOUNT",
      "STORE_ID",
      "menura.row_identity",
      "ORDERS_STORES.REGION",
      "ORDERS_STORES.STORE_ID"
    FROM "menura_7_semijoin"
    LEFT JOIN "menura_9_filter" ON "menura_7_semijoin"."STORE_ID" = "menura_9_filter"."ORDERS_STORES.STORE_ID"
  ),
  "menura_11_deduplicate" AS (
    SELECT
      "AMOUNT",
      "STORE_ID",
      "menura.row_identity",
      "ORDERS_STORES.REGION",
      "ORDERS_STORES.STORE_ID"
    FROM "menura_10_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_STORES.REGION", "menura.row_identity") = 1
  ),
  "menura_12_aggregate" AS (
    SELECT
      "ORDERS_STORES.REGION" AS "dimension_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_1_component_1"
    FROM "menura_11_deduplicate"
    GROUP BY 1
  ),
  "menura_13_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_12_aggregate"
  )
SELECT
  "region",
  "total_amount"
FROM "menura_13_project"