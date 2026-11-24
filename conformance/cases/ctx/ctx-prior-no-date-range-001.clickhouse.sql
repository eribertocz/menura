WITH
  "menura_1_scan" AS (
    SELECT
      "order_date" AS "ORDER_DATE",
      "order_id" AS "ORDER_ID"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_DATE")) AS "dimension_1",
      accurateCast(coalesce(count("ORDER_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1",
      accurateCast(coalesce(count("ORDER_ID"), 0), 'Nullable(Int64)') AS "metric_2_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST(CAST(addYears("dimension_1", toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)) AS Nullable(Date32)) AS "context_1_shifted_time",
      CAST(TRUE AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_2_aggregate"
  ),
  "menura_4_scan" AS (
    SELECT
      "order_date" AS "ORDER_DATE",
      "order_id" AS "ORDER_ID"
    FROM "shop"."orders"
  ),
  "menura_5_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_DATE")) AS "dimension_1",
      accurateCast(coalesce(count("ORDER_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_4_scan"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_5_aggregate"
  ),
  "menura_7_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_1_time_present",
      TRUE AS "menura.present"
    FROM "menura_6_project"
  ),
  "menura_8_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "context_1_component_1"
    FROM "menura_3_project" AS "menura_result"
    LEFT JOIN "menura_7_present" AS "menura_context" ON isNotDistinctFrom("menura_result"."context_1_shifted_time", "menura_context"."dimension_1") AND isNotDistinctFrom("menura_result"."context_1_time_present", "menura_context"."context_1_time_present")
  ),
  "menura_9_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "order_count",
      "context_1_component_1" AS "order_count_prior_year"
    FROM "menura_8_lookup"
  )
SELECT
  "order_date",
  "order_count",
  "order_count_prior_year"
FROM "menura_9_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1