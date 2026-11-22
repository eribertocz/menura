WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "status" AS "STATUS",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."main"."orders"
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
    FROM "shop"."main"."customers"
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
  "menura_5_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_4_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_6_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.REGION" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_5_deduplicate"
    GROUP BY 1
  ),
  "menura_7_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "status" AS "STATUS"
    FROM "shop"."main"."orders"
  ),
  "menura_8_filter" AS (
    SELECT
      "AMOUNT",
      "STATUS"
    FROM "menura_7_scan"
    WHERE TRUE AND ("STATUS" <> 'void')
  ),
  "menura_9_aggregate" AS (
    SELECT
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_8_filter"
  ),
  "menura_10_present" AS (
    SELECT
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_9_aggregate"
  ),
  "menura_11_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_6_aggregate" AS "menura_result"
    LEFT JOIN "menura_10_present" AS "menura_context" ON TRUE
  ),
  "menura_12_project" AS (
    SELECT
      "dimension_1" AS "region",
      CAST(CASE WHEN CAST("context_1_component_1" AS DOUBLE) = 0 THEN NULL ELSE CAST("metric_1_component_1" AS DOUBLE) / CAST("context_1_component_1" AS DOUBLE) END AS DOUBLE) AS "sales_share_of_regions",
      CAST(CASE WHEN CAST("context_1_component_1" AS DOUBLE) = 0 THEN NULL ELSE CAST("metric_2_component_1" AS DOUBLE) / CAST("context_1_component_1" AS DOUBLE) END AS DOUBLE) AS "sales_share_of_all"
    FROM "menura_11_lookup"
  )
SELECT
  "region",
  "sales_share_of_regions",
  "sales_share_of_all"
FROM "menura_12_project"