WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "segment" AS "SEGMENT"
    FROM "shop"."customers"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "SEGMENT" AS "dimension_1",
      CAST(COUNT("CUSTOMER_ID") AS BIGINT) AS "metric_2_component_1",
      CAST(COUNT("CUSTOMER_ID") AS BIGINT) AS "metric_3_component_2"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "store_id" AS "STORE_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_4_scan" AS (
    SELECT
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_5_scan" AS (
    SELECT
      "region" AS "REGION",
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_6_filter" AS (
    SELECT
      "REGION",
      "STORE_ID"
    FROM "menura_5_scan"
    WHERE ("REGION" = 'NORTH') AND TRUE
  ),
  "menura_7_related" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_4_scan"
    GROUP BY 1
  ),
  "menura_8_visible" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_6_filter"
    GROUP BY 1
  ),
  "menura_9_semijoin" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."CUSTOMER_ID",
      "menura_row"."STORE_ID",
      "menura_row"."menura.row_identity"
    FROM "menura_3_scan" AS "menura_row"
    JOIN "menura_7_related" AS "menura_related" ON "menura_row"."STORE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_8_visible" AS "menura_visible" ON "menura_row"."STORE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_10_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "segment" AS "ORDERS_CUSTOMERS.SEGMENT"
    FROM "shop"."customers"
  ),
  "menura_11_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STORE_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT"
    FROM "menura_9_semijoin"
    LEFT JOIN "menura_10_scan" ON "menura_9_semijoin"."CUSTOMER_ID" = "menura_10_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_12_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STORE_ID",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.SEGMENT"
    FROM "menura_11_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.SEGMENT", "menura.row_identity") = 1
  ),
  "menura_13_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.SEGMENT" AS "dimension_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_3_component_1"
    FROM "menura_12_deduplicate"
    GROUP BY 1
  ),
  "menura_14_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      "metric_3_component_2",
      TRUE AS "menura.present"
    FROM "menura_2_aggregate"
  ),
  "menura_15_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "metric_3_component_1",
      TRUE AS "menura.present"
    FROM "menura_13_aggregate"
  ),
  "menura_16_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_2_aggregate"
    UNION
    SELECT
      "dimension_1"
    FROM "menura_13_aggregate"
  ),
  "menura_17_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE CAST(0 AS BIGINT) END AS "metric_2_component_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_3_component_2" ELSE CAST(0 AS BIGINT) END AS "metric_3_component_2",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_3_component_1" ELSE NULL END AS "metric_3_component_1"
    FROM "menura_16_keys" AS "menura_keys"
    LEFT JOIN "menura_14_present" AS "menura_branch_1" ON "menura_keys"."dimension_1" IS NOT DISTINCT FROM "menura_branch_1"."dimension_1"
    LEFT JOIN "menura_15_present" AS "menura_branch_2" ON "menura_keys"."dimension_1" IS NOT DISTINCT FROM "menura_branch_2"."dimension_1"
  ),
  "menura_18_project" AS (
    SELECT
      "dimension_1" AS "segment",
      "metric_1_component_1" AS "total_amount",
      "metric_2_component_1" AS "customer_count",
      CAST(CASE WHEN CAST("metric_3_component_2" AS DOUBLE) = 0 THEN NULL ELSE CAST("metric_3_component_1" AS DOUBLE) / CAST("metric_3_component_2" AS DOUBLE) END AS DOUBLE) AS "amount_per_customer"
    FROM "menura_17_stitch"
  )
SELECT
  "segment",
  "total_amount",
  "customer_count",
  "amount_per_customer"
FROM "menura_18_project"