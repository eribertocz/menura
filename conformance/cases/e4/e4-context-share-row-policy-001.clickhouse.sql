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
  "menura_5_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_4_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.REGION", "STATUS", "menura.row_identity") = 1
  ),
  "menura_6_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.REGION" AS "dimension_1",
      "STATUS" AS "dimension_2",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_5_deduplicate"
    GROUP BY 1, 2
  ),
  "menura_7_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "status" AS "STATUS",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_8_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity"
    FROM "menura_7_scan"
    WHERE TRUE AND ("STATUS" <> 'void')
  ),
  "menura_9_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "ORDERS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_10_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_8_filter"
    LEFT JOIN "menura_9_scan" ON "menura_8_filter"."CUSTOMER_ID" = "menura_9_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_11_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_10_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_12_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.REGION" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_11_deduplicate"
    GROUP BY 1
  ),
  "menura_13_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_12_aggregate"
  ),
  "menura_14_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."dimension_2" AS "dimension_2",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_6_aggregate" AS "menura_result"
    LEFT JOIN "menura_13_present" AS "menura_context" ON isNotDistinctFrom("menura_result"."dimension_1", "menura_context"."dimension_1")
  ),
  "menura_15_project" AS (
    SELECT
      "dimension_1" AS "region",
      "dimension_2" AS "status",
      "metric_1_component_1" AS "sales",
      CAST(CASE WHEN toFloat64("context_1_component_1") = 0 THEN NULL ELSE toFloat64("metric_2_component_1") / toFloat64("context_1_component_1") END AS Nullable(Float64)) AS "sales_share_in_region"
    FROM "menura_14_lookup"
  )
SELECT
  "region",
  "status",
  "sales",
  "sales_share_in_region"
FROM "menura_15_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1