WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_1_scan"
    WHERE ("ORDER_DATE" >= {p1:Date32}) AND ("ORDER_DATE" <= {p2:Date32})
  ),
  "menura_3_aggregate" AS (
    SELECT
      toDate32(toMonday("ORDER_DATE")) AS "dimension_1",
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
      CAST(CAST(addDays(toDate32(toMonday(CAST(addYears(CAST(addDays(CAST(addDays("dimension_1", toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays("dimension_1", toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)))), toInt64(accurateCast(toInt256(toInt64(7)) * toInt256(accurateCast(toInt256(accurateCast(CASE WHEN accurateCast(toISOWeek("dimension_1"), 'Nullable(Int64)') IS NULL OR accurateCast(toISOWeek(CAST(addDays(CAST(addYears(CAST(addYears(CAST(addDays(CAST(addDays("dimension_1", toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays("dimension_1", toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(7)), 'Nullable(Int128)'))) AS Nullable(Date32))), 'Nullable(Int64)') IS NULL THEN NULL ELSE least(accurateCast(accurateCast(toISOWeek("dimension_1"), 'Nullable(Int64)'), 'Nullable(Int64)'), accurateCast(accurateCast(toISOWeek(CAST(addDays(CAST(addYears(CAST(addYears(CAST(addDays(CAST(addDays("dimension_1", toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays("dimension_1", toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(7)), 'Nullable(Int128)'))) AS Nullable(Date32))), 'Nullable(Int64)'), 'Nullable(Int64)')) END, 'Nullable(Int64)')) - toInt256(toInt64(1)), 'Nullable(Int128)')), 'Nullable(Int128)'))) AS Nullable(Date32)) AS Nullable(Date32)) AS "context_1_shifted_time",
      CAST(TRUE AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_3_aggregate"
  ),
  "menura_5_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_6_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_5_scan"
    WHERE (("ORDER_DATE" >= CAST(addDays(toDate32(toMonday(CAST(addYears(CAST(addDays(CAST(addDays(toDate32(toMonday({p1:Date32})), toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays(toDate32(toMonday({p3:Date32})), toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)))), toInt64(accurateCast(toInt256(toInt64(7)) * toInt256(accurateCast(toInt256(accurateCast(CASE WHEN accurateCast(toISOWeek(toDate32(toMonday({p4:Date32}))), 'Nullable(Int64)') IS NULL OR accurateCast(toISOWeek(CAST(addDays(CAST(addYears(CAST(addYears(CAST(addDays(CAST(addDays(toDate32(toMonday({p5:Date32})), toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays(toDate32(toMonday({p6:Date32})), toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(7)), 'Nullable(Int128)'))) AS Nullable(Date32))), 'Nullable(Int64)') IS NULL THEN NULL ELSE least(accurateCast(accurateCast(toISOWeek(toDate32(toMonday({p4:Date32}))), 'Nullable(Int64)'), 'Nullable(Int64)'), accurateCast(accurateCast(toISOWeek(CAST(addDays(CAST(addYears(CAST(addYears(CAST(addDays(CAST(addDays(toDate32(toMonday({p5:Date32})), toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays(toDate32(toMonday({p6:Date32})), toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(7)), 'Nullable(Int128)'))) AS Nullable(Date32))), 'Nullable(Int64)'), 'Nullable(Int64)')) END, 'Nullable(Int64)')) - toInt256(toInt64(1)), 'Nullable(Int128)')), 'Nullable(Int128)'))) AS Nullable(Date32))) AND ("ORDER_DATE" < CAST(addWeeks(CAST(addDays(toDate32(toMonday(CAST(addYears(CAST(addDays(CAST(addDays(toDate32(toMonday({p7:Date32})), toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays(toDate32(toMonday({p8:Date32})), toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)))), toInt64(accurateCast(toInt256(toInt64(7)) * toInt256(accurateCast(toInt256(accurateCast(CASE WHEN accurateCast(toISOWeek(toDate32(toMonday({p9:Date32}))), 'Nullable(Int64)') IS NULL OR accurateCast(toISOWeek(CAST(addDays(CAST(addYears(CAST(addYears(CAST(addDays(CAST(addDays(toDate32(toMonday({p10:Date32})), toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays(toDate32(toMonday({p11:Date32})), toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(7)), 'Nullable(Int128)'))) AS Nullable(Date32))), 'Nullable(Int64)') IS NULL THEN NULL ELSE least(accurateCast(accurateCast(toISOWeek(toDate32(toMonday({p9:Date32}))), 'Nullable(Int64)'), 'Nullable(Int64)'), accurateCast(accurateCast(toISOWeek(CAST(addDays(CAST(addYears(CAST(addYears(CAST(addDays(CAST(addDays(toDate32(toMonday({p10:Date32})), toInt64(toInt64(3))) AS Nullable(Date32)), toInt64(accurateCast(toInt256(toInt64(4)) - toInt256(accurateCast(toDayOfYear(CAST(addDays(toDate32(toMonday({p11:Date32})), toInt64(toInt64(3))) AS Nullable(Date32))), 'Nullable(Int64)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(7)), 'Nullable(Int128)'))) AS Nullable(Date32))), 'Nullable(Int64)'), 'Nullable(Int64)')) END, 'Nullable(Int64)')) - toInt256(toInt64(1)), 'Nullable(Int128)')), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32))))
  ),
  "menura_7_aggregate" AS (
    SELECT
      toDate32(toMonday("ORDER_DATE")) AS "dimension_1",
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
  decimal_check_overflow = 1