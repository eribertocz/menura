WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "status" AS "STATUS",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity"
    FROM "menura_1_scan"
    WHERE TRUE AND ("STATUS" <> 'void')
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_4_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "region" AS "REGION"
    FROM "shop"."customers"
  ),
  "menura_5_filter" AS (
    SELECT
      "CUSTOMER_ID",
      "REGION"
    FROM "menura_4_scan"
    WHERE ("REGION" IN (SELECT unnest(CAST($1 AS VARCHAR[])))) AND TRUE
  ),
  "menura_6_related" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_3_scan"
    GROUP BY 1
  ),
  "menura_7_visible" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_5_filter"
    GROUP BY 1
  ),
  "menura_8_semijoin" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."CUSTOMER_ID",
      "menura_row"."STATUS",
      "menura_row"."menura.row_identity"
    FROM "menura_2_filter" AS "menura_row"
    JOIN "menura_6_related" AS "menura_related" ON CAST("menura_row"."CUSTOMER_ID" AS HUGEINT) = CAST("menura_related"."menura.key_1" AS HUGEINT)
    LEFT JOIN "menura_7_visible" AS "menura_visible" ON CAST("menura_row"."CUSTOMER_ID" AS HUGEINT) = CAST("menura_visible"."menura.key_1" AS HUGEINT)
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_9_scan" AS (
    SELECT
      "city" AS "ORDERS_CUSTOMERS.CITY",
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "ORDERS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_10_filter" AS (
    SELECT
      "ORDERS_CUSTOMERS.CITY",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_9_scan"
    WHERE ("ORDERS_CUSTOMERS.REGION" IN (SELECT unnest(CAST($1 AS VARCHAR[])))) AND TRUE
  ),
  "menura_11_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CITY",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_8_semijoin"
    LEFT JOIN "menura_10_filter" ON CAST("menura_8_semijoin"."CUSTOMER_ID" AS HUGEINT) = CAST("menura_10_filter"."ORDERS_CUSTOMERS.CUSTOMER_ID" AS HUGEINT)
  ),
  "menura_12_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CITY",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_11_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.CITY", "menura.row_identity") = 1
  ),
  "menura_13_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.CITY" AS "dimension_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_2_component_1"
    FROM "menura_12_deduplicate"
    GROUP BY 1
  ),
  "menura_14_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."refunds"
  ),
  "menura_15_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_16_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "region" AS "REGION"
    FROM "shop"."customers"
  ),
  "menura_17_filter" AS (
    SELECT
      "CUSTOMER_ID",
      "REGION"
    FROM "menura_16_scan"
    WHERE ("REGION" IN (SELECT unnest(CAST($1 AS VARCHAR[])))) AND TRUE
  ),
  "menura_18_related" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_15_scan"
    GROUP BY 1
  ),
  "menura_19_visible" AS (
    SELECT
      "CUSTOMER_ID" AS "menura.key_1",
      COUNT(*) AS "menura.rows"
    FROM "menura_17_filter"
    GROUP BY 1
  ),
  "menura_20_semijoin" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."CUSTOMER_ID",
      "menura_row"."menura.row_identity"
    FROM "menura_14_scan" AS "menura_row"
    JOIN "menura_18_related" AS "menura_related" ON "menura_row"."CUSTOMER_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_19_visible" AS "menura_visible" ON "menura_row"."CUSTOMER_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_21_scan" AS (
    SELECT
      "city" AS "REFUNDS_CUSTOMERS.CITY",
      "customer_id" AS "REFUNDS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "REFUNDS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_22_filter" AS (
    SELECT
      "REFUNDS_CUSTOMERS.CITY",
      "REFUNDS_CUSTOMERS.CUSTOMER_ID",
      "REFUNDS_CUSTOMERS.REGION"
    FROM "menura_21_scan"
    WHERE ("REFUNDS_CUSTOMERS.REGION" IN (SELECT unnest(CAST($1 AS VARCHAR[])))) AND TRUE
  ),
  "menura_23_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "menura.row_identity",
      "REFUNDS_CUSTOMERS.CITY",
      "REFUNDS_CUSTOMERS.CUSTOMER_ID",
      "REFUNDS_CUSTOMERS.REGION"
    FROM "menura_20_semijoin"
    LEFT JOIN "menura_22_filter" ON "menura_20_semijoin"."CUSTOMER_ID" = "menura_22_filter"."REFUNDS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_24_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "menura.row_identity",
      "REFUNDS_CUSTOMERS.CITY",
      "REFUNDS_CUSTOMERS.CUSTOMER_ID",
      "REFUNDS_CUSTOMERS.REGION"
    FROM "menura_23_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "REFUNDS_CUSTOMERS.CITY", "menura.row_identity") = 1
  ),
  "menura_25_aggregate" AS (
    SELECT
      "REFUNDS_CUSTOMERS.CITY" AS "dimension_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_1_component_1"
    FROM "menura_24_deduplicate"
    GROUP BY 1
  ),
  "menura_26_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_13_aggregate"
  ),
  "menura_27_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_25_aggregate"
  ),
  "menura_28_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_13_aggregate"
    UNION
    SELECT
      "dimension_1"
    FROM "menura_25_aggregate"
  ),
  "menura_29_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1"
    FROM "menura_28_keys" AS "menura_keys"
    LEFT JOIN "menura_26_present" AS "menura_branch_1" ON "menura_keys"."dimension_1" IS NOT DISTINCT FROM "menura_branch_1"."dimension_1"
    LEFT JOIN "menura_27_present" AS "menura_branch_2" ON "menura_keys"."dimension_1" IS NOT DISTINCT FROM "menura_branch_2"."dimension_1"
  ),
  "menura_30_project" AS (
    SELECT
      "dimension_1" AS "city",
      "metric_1_component_1" AS "refund_total",
      "metric_2_component_1" AS "order_total"
    FROM "menura_29_stitch"
  ),
  "menura_31_sort" AS (
    SELECT
      "city",
      "refund_total",
      "order_total"
    FROM "menura_30_project"
    ORDER BY "city" ASC NULLS FIRST
  ),
  "menura_32_limit" AS (
    SELECT
      "city",
      "refund_total",
      "order_total"
    FROM "menura_31_sort"
    ORDER BY "city" ASC NULLS FIRST
    LIMIT 2
  )
SELECT
  "city",
  "refund_total",
  "order_total"
FROM "menura_32_limit"
ORDER BY "city" ASC NULLS FIRST