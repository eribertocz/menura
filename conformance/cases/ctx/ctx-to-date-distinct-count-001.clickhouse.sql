WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "order_date" AS "ORDER_DATE",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "ORDERS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_3_join" AS (
    SELECT
      "CUSTOMER_ID",
      "ORDER_DATE",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "ORDER_DATE",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY toDate32(toStartOfMonth("ORDER_DATE")), "ORDERS_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_DATE")) AS "dimension_1",
      "ORDERS_CUSTOMERS.REGION" AS "dimension_2",
      accurateCast(coalesce(count(DISTINCT toInt256("CUSTOMER_ID")), 0), 'Nullable(Int64)') AS "metric_1_component_1",
      accurateCast(coalesce(count(DISTINCT toInt256("CUSTOMER_ID")), 0), 'Nullable(Int64)') AS "metric_2_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1, 2
  ),
  "menura_6_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "order_date" AS "ORDER_DATE",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_7_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "region" AS "ORDERS_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_8_join" AS (
    SELECT
      "CUSTOMER_ID",
      "ORDER_DATE",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION"
    FROM "menura_6_scan"
    LEFT JOIN "menura_7_scan" ON "menura_6_scan"."CUSTOMER_ID" = "menura_7_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_9_todate" AS (
    SELECT
      "menura_rows"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_rows"."ORDER_DATE" AS "ORDER_DATE",
      "menura_rows"."menura.row_identity" AS "menura.row_identity",
      "menura_rows"."ORDERS_CUSTOMERS.CUSTOMER_ID" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "menura_rows"."ORDERS_CUSTOMERS.REGION" AS "ORDERS_CUSTOMERS.REGION",
      "menura_start" AS "menura.period"
    FROM "menura_8_join" AS "menura_rows"
    ARRAY JOIN arrayMap(menura_offset -> addMonths(assumeNotNull(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE"))), menura_offset), range(toUInt64(if(ifNull(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE"))), addYears(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE")), 1)) < 0, 1), 0, intDiv(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE"))), addYears(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE")), 1)), 1) + 1)))) AS "menura_start"
    WHERE "menura_rows"."ORDER_DATE" IS NOT NULL AND toDate32(toStartOfYear("menura_start")) = toDate32(toStartOfYear(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE"))))
  ),
  "menura_10_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "ORDER_DATE",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.REGION",
      "menura.period"
    FROM "menura_9_todate"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY toDate32(toStartOfMonth("menura.period")), "ORDERS_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_11_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("menura.period")) AS "dimension_1",
      "ORDERS_CUSTOMERS.REGION" AS "dimension_2",
      accurateCast(coalesce(count(DISTINCT toInt256("CUSTOMER_ID")), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_10_deduplicate"
    GROUP BY 1, 2
  ),
  "menura_12_present" AS (
    SELECT
      "dimension_1",
      "dimension_2",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_11_aggregate"
  ),
  "menura_13_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."dimension_2" AS "dimension_2",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "context_1_component_1"
    FROM "menura_5_aggregate" AS "menura_result"
    LEFT JOIN "menura_12_present" AS "menura_context" ON isNotDistinctFrom("menura_result"."dimension_1", "menura_context"."dimension_1") AND isNotDistinctFrom("menura_result"."dimension_2", "menura_context"."dimension_2")
  ),
  "menura_14_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "dimension_2" AS "region",
      "metric_1_component_1" AS "buyers",
      "context_1_component_1" AS "buyers_ytd"
    FROM "menura_13_lookup"
  )
SELECT
  "order_date",
  "region",
  "buyers",
  "buyers_ytd"
FROM "menura_14_project"
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