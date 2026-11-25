WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "order_id" AS "ORDER_ID",
      "status" AS "STATUS",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "ORDER_ID",
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
      "ORDER_ID",
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
      "ORDER_ID",
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
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_2_component_1",
      accurateCast(coalesce(count("ORDER_ID"), 0), 'Nullable(Int64)') AS "metric_3_component_1"
    FROM "menura_5_deduplicate"
    GROUP BY 1
  ),
  "menura_7_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "status" AS "STATUS"
    FROM "shop"."orders"
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
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
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
      "menura_result"."metric_3_component_1" AS "metric_3_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_6_aggregate" AS "menura_result"
    LEFT JOIN "menura_10_present" AS "menura_context" ON TRUE
  ),
  "menura_12_scan" AS (
    SELECT
      "order_id" AS "ORDER_ID",
      "status" AS "STATUS"
    FROM "shop"."orders"
  ),
  "menura_13_filter" AS (
    SELECT
      "ORDER_ID",
      "STATUS"
    FROM "menura_12_scan"
    WHERE TRUE AND ("STATUS" <> 'void')
  ),
  "menura_14_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("ORDER_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_13_filter"
  ),
  "menura_15_present" AS (
    SELECT
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_14_aggregate"
  ),
  "menura_16_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."metric_3_component_1" AS "metric_3_component_1",
      "menura_result"."context_1_component_1" AS "context_1_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "context_2_component_1"
    FROM "menura_11_lookup" AS "menura_result"
    LEFT JOIN "menura_15_present" AS "menura_context" ON TRUE
  ),
  "menura_17_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "sales",
      CAST(CASE WHEN toFloat64("context_1_component_1") = 0 THEN NULL ELSE toFloat64("metric_2_component_1") / toFloat64("context_1_component_1") END AS Nullable(Float64)) AS "sales_share_of_all",
      CAST(CASE WHEN toFloat64("context_2_component_1") = 0 THEN NULL ELSE toFloat64("metric_3_component_1") / toFloat64("context_2_component_1") END AS Nullable(Float64)) AS "order_count_share_of_all"
    FROM "menura_16_lookup"
  )
SELECT
  "region",
  "sales",
  "sales_share_of_all",
  "order_count_share_of_all"
FROM "menura_17_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1,
  use_variant_as_common_type = 0