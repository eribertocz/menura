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
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_6_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.REGION" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_5_deduplicate"
    GROUP BY 1
  ),
  "menura_7_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "refund" AS "REFUND",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."returns"
  ),
  "menura_8_scan" AS (
    SELECT
      "customer_id" AS "RETURNS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "RETURNS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_9_join" AS (
    SELECT
      "CUSTOMER_ID",
      "REFUND",
      "menura.row_identity",
      "RETURNS_CUSTOMERS.CUSTOMER_ID",
      "RETURNS_CUSTOMERS.REGION"
    FROM "menura_7_scan"
    LEFT JOIN "menura_8_scan" ON "menura_7_scan"."CUSTOMER_ID" = "menura_8_scan"."RETURNS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_10_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "REFUND",
      "menura.row_identity",
      "RETURNS_CUSTOMERS.CUSTOMER_ID",
      "RETURNS_CUSTOMERS.REGION"
    FROM "menura_9_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "RETURNS_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_11_aggregate" AS (
    SELECT
      "RETURNS_CUSTOMERS.REGION" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("REFUND")), 'Nullable(Int128)') AS "metric_1_component_2",
      accurateCast(sumOrNull(toInt256("REFUND")), 'Nullable(Int128)') AS "metric_2_component_2"
    FROM "menura_10_deduplicate"
    GROUP BY 1
  ),
  "menura_12_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_6_aggregate"
  ),
  "menura_13_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_2",
      "metric_2_component_2",
      TRUE AS "menura.present"
    FROM "menura_11_aggregate"
  ),
  "menura_14_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_6_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_11_aggregate"
  ),
  "menura_15_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_2" ELSE NULL END AS "metric_1_component_2",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_2_component_2" ELSE NULL END AS "metric_2_component_2"
    FROM "menura_14_keys" AS "menura_keys"
    LEFT JOIN "menura_12_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_13_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_16_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "status" AS "STATUS"
    FROM "shop"."orders"
  ),
  "menura_17_filter" AS (
    SELECT
      "AMOUNT",
      "STATUS"
    FROM "menura_16_scan"
    WHERE TRUE AND ("STATUS" <> 'void')
  ),
  "menura_18_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_17_filter"
  ),
  "menura_19_scan" AS (
    SELECT
      "refund" AS "REFUND"
    FROM "shop"."returns"
  ),
  "menura_20_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("REFUND")), 'Nullable(Int128)') AS "metric_1_component_2"
    FROM "menura_19_scan"
  ),
  "menura_21_present" AS (
    SELECT
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_18_aggregate"
  ),
  "menura_22_present" AS (
    SELECT
      "metric_1_component_2",
      TRUE AS "menura.present"
    FROM "menura_20_aggregate"
  ),
  "menura_23_stitch" AS (
    SELECT
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_2" ELSE NULL END AS "metric_1_component_2"
    FROM "menura_21_present" AS "menura_branch_1"
    LEFT JOIN "menura_22_present" AS "menura_branch_2" ON TRUE
  ),
  "menura_24_present" AS (
    SELECT
      "metric_1_component_1",
      "metric_1_component_2",
      TRUE AS "menura.present"
    FROM "menura_23_stitch"
  ),
  "menura_25_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."metric_1_component_2" AS "metric_1_component_2",
      "menura_result"."metric_2_component_2" AS "metric_2_component_2",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_2" ELSE NULL END AS "context_1_component_2"
    FROM "menura_15_stitch" AS "menura_result"
    LEFT JOIN "menura_24_present" AS "menura_context" ON TRUE
  ),
  "menura_26_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      "metric_1_component_2" AS "metric_1_component_2",
      "metric_2_component_2" AS "metric_2_component_2",
      "context_1_component_1" AS "context_1_component_1",
      "context_1_component_2" AS "context_1_component_2",
      accurateCast(accurateCast(toInt256("metric_2_component_1") - toInt256("metric_2_component_2"), 'Nullable(Int128)'), 'Nullable(Int128)') AS "metric_2_input_1",
      accurateCast(accurateCast(toInt256("context_1_component_1") - toInt256("context_1_component_2"), 'Nullable(Int128)'), 'Nullable(Int128)') AS "metric_2_input_2"
    FROM "menura_25_lookup"
  ),
  "menura_27_project" AS (
    SELECT
      "dimension_1" AS "region",
      accurateCast(accurateCast(toInt256("metric_1_component_1") - toInt256("metric_1_component_2"), 'Nullable(Int128)'), 'Nullable(Int128)') AS "net_sales",
      CAST(CASE WHEN toFloat64("metric_2_input_2") = 0 THEN NULL ELSE toFloat64("metric_2_input_1") / toFloat64("metric_2_input_2") END AS Nullable(Float64)) AS "net_sales_share_of_regions"
    FROM "menura_26_project"
  )
SELECT
  "region",
  "net_sales",
  "net_sales_share_of_regions"
FROM "menura_27_project"
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