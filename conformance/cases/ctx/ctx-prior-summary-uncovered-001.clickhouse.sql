WITH
  "menura_1_scan" AS (
    SELECT
      "amount_sum" AS "AMOUNT_SUM",
      "order_month" AS "ORDER_MONTH"
    FROM "shop"."orders_by_region_month"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH"
    FROM "menura_2_filter"
    WHERE ("ORDER_MONTH" >= {p1:Date32}) AND ("ORDER_MONTH" <= {p2:Date32})
  ),
  "menura_4_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_MONTH")) AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT_SUM")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("AMOUNT_SUM")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST(CAST(addMonths("dimension_1", toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)) AS Nullable(Date32)) AS "context_1_shifted_time",
      CAST(TRUE AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_4_aggregate"
  ),
  "menura_6_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_7_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_6_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_8_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_7_filter"
    WHERE (("ORDER_DATE" >= CAST(addMonths(toDate32(toStartOfMonth({p1:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32))) AND ("ORDER_DATE" < CAST(addMonths(CAST(addMonths(toDate32(toStartOfMonth({p2:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32))))
  ),
  "menura_9_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_DATE")) AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_8_filter"
    GROUP BY 1
  ),
  "menura_10_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_9_aggregate"
  ),
  "menura_11_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_1_time_present",
      TRUE AS "menura.present"
    FROM "menura_10_project"
  ),
  "menura_12_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_5_project" AS "menura_result"
    LEFT JOIN "menura_11_present" AS "menura_context" ON isNotDistinctFrom("menura_result"."context_1_shifted_time", "menura_context"."dimension_1") AND isNotDistinctFrom("menura_result"."context_1_time_present", "menura_context"."context_1_time_present")
  ),
  "menura_13_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "total_amount",
      "context_1_component_1" AS "total_amount_prior_month"
    FROM "menura_12_lookup"
  )
SELECT
  "order_date",
  "total_amount",
  "total_amount_prior_month"
FROM "menura_13_project"
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