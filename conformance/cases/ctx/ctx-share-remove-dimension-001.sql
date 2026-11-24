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
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "ORDERS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_4_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_2_filter"
    LEFT JOIN "menura_3_scan" ON "menura_2_filter"."CUSTOMER_ID" = "menura_3_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_5_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_4_join"
    WHERE "AMOUNT" > CAST($1 AS BIGINT)
  ),
  "menura_6_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_5_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.REGION", "STATUS", "menura.row_identity") = 1
  ),
  "menura_7_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.REGION" AS "dimension_1",
      "STATUS" AS "dimension_2",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_6_deduplicate"
    GROUP BY 1, 2
  ),
  "menura_8_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "status" AS "STATUS",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_9_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity"
    FROM "menura_8_scan"
    WHERE TRUE AND ("STATUS" <> 'void')
  ),
  "menura_10_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "ORDERS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_11_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_9_filter"
    LEFT JOIN "menura_10_scan" ON "menura_9_filter"."CUSTOMER_ID" = "menura_10_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_12_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_11_join"
    WHERE "AMOUNT" > CAST($1 AS BIGINT)
  ),
  "menura_13_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_12_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_14_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.REGION" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_13_deduplicate"
    GROUP BY 1
  ),
  "menura_15_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_14_aggregate"
  ),
  "menura_16_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."dimension_2" AS "dimension_2",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_7_aggregate" AS "menura_result"
    LEFT JOIN "menura_15_present" AS "menura_context" ON "menura_result"."dimension_1" IS NOT DISTINCT FROM "menura_context"."dimension_1"
  ),
  "menura_17_project" AS (
    SELECT
      "dimension_1" AS "region",
      "dimension_2" AS "status",
      "metric_1_component_1" AS "sales",
      CAST(CASE WHEN CAST("context_1_component_1" AS DOUBLE) = 0 THEN NULL ELSE CAST("metric_2_component_1" AS DOUBLE) / CAST("context_1_component_1" AS DOUBLE) END AS DOUBLE) AS "sales_share_in_region"
    FROM "menura_16_lookup"
  )
SELECT
  "region",
  "status",
  "sales",
  "sales_share_in_region"
FROM "menura_17_project"