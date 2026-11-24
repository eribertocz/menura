WITH
  "menura_1_scan" AS (
    SELECT
      "age" AS "AGE",
      "region" AS "REGION"
    FROM "shop"."customers"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      CAST(CASE WHEN coalesce(count("AGE"), 0) = 0 THEN NULL ELSE toFloat64(sum(toInt256("AGE"))) / toFloat64(count("AGE")) END AS Nullable(Float64)) AS "metric_2_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "status" AS "STATUS",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_4_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "ORDERS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_5_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_3_scan"
    LEFT JOIN "menura_4_scan" ON "menura_3_scan"."CUSTOMER_ID" = "menura_4_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_6_deduplicate" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "STATUS",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_5_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_7_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.REGION" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(coalesce(count(DISTINCT toInt256("CUSTOMER_ID")), 0), 'Nullable(Int64)') AS "metric_3_component_1",
      accurateCast(sumOrNull(toInt256(if(ifNull("STATUS" = 'OPEN', 0), "AMOUNT", NULL))), 'Nullable(Int128)') AS "metric_4_component_1"
    FROM "menura_6_deduplicate"
    GROUP BY 1
  ),
  "menura_8_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_2_aggregate"
  ),
  "menura_9_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "metric_3_component_1",
      "metric_4_component_1",
      TRUE AS "menura.present"
    FROM "menura_7_aggregate"
  ),
  "menura_10_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_2_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_7_aggregate"
  ),
  "menura_11_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_3_component_1" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "metric_3_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_4_component_1" ELSE NULL END AS "metric_4_component_1"
    FROM "menura_10_keys" AS "menura_keys"
    LEFT JOIN "menura_8_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_9_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_12_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "total_amount",
      "metric_2_component_1" AS "average_age",
      "metric_3_component_1" AS "ordering_customer_count",
      "metric_4_component_1" AS "open_amount"
    FROM "menura_11_stitch"
  )
SELECT
  "region",
  "total_amount",
  "average_age",
  "ordering_customer_count",
  "open_amount"
FROM "menura_12_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1