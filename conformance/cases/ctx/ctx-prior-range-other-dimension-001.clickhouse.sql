WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "ordered_at" AS "ORDERED_AT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "ORDERED_AT",
      "ORDER_DATE"
    FROM "menura_1_scan"
    WHERE ("ORDERED_AT" >= toTimeZone(toDateTime64({p1:Date32}, 6, {p2:String}), 'UTC')) AND ("ORDERED_AT" < toTimeZone(toDateTime64({p3:Date32}, 6, {p2:String}), 'UTC'))
  ),
  "menura_3_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_DATE")) AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST(CAST(addYears("dimension_1", toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)) AS Nullable(Date32)) AS "context_1_shifted_time",
      CAST(TRUE AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_3_aggregate"
  ),
  "menura_5_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "ordered_at" AS "ORDERED_AT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_6_filter" AS (
    SELECT
      "AMOUNT",
      "ORDERED_AT",
      "ORDER_DATE"
    FROM "menura_5_scan"
    WHERE ("ORDERED_AT" >= toTimeZone(toDateTime64({p1:Date32}, 6, {p2:String}), 'UTC')) AND ("ORDERED_AT" < toTimeZone(toDateTime64({p3:Date32}, 6, {p2:String}), 'UTC'))
  ),
  "menura_7_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_DATE")) AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_6_filter"
    GROUP BY 1
  ),
  "menura_8_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_7_aggregate"
  ),
  "menura_9_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_1_time_present",
      TRUE AS "menura.present"
    FROM "menura_8_project"
  ),
  "menura_10_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_4_project" AS "menura_result"
    LEFT JOIN "menura_9_present" AS "menura_context" ON isNotDistinctFrom("menura_result"."context_1_shifted_time", "menura_context"."dimension_1") AND isNotDistinctFrom("menura_result"."context_1_time_present", "menura_context"."context_1_time_present")
  ),
  "menura_11_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "sales",
      "context_1_component_1" AS "sales_prior_year"
    FROM "menura_10_lookup"
  )
SELECT
  "order_date",
  "sales",
  "sales_prior_year"
FROM "menura_11_project"
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