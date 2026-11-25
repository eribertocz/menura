WITH
  "menura_1_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_2_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD"
    FROM "menura_1_scan"
    WHERE ("PERIOD" >= {p1:Date32}) AND ("PERIOD" <= {p2:Date32})
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_4_periods" AS (
    SELECT
      "menura_start" AS "period"
    FROM (SELECT arrayJoin(arrayMap(menura_offset -> addMonths(assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), menura_offset), range(toUInt64(if(ifNull(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), {p2:Date32}) < 0, 1), 0, intDiv(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), {p2:Date32}), 1) + 1))))) AS "menura_start") AS "menura_periods"
  ),
  "menura_5_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_3_scan" AS "menura_row"
    CROSS JOIN "menura_4_periods" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_6_values" AS (
    SELECT
      toDate32(toStartOfMonth("PERIOD")) AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1",
      sumOrNull(toInt256("BALANCE")) AS "metric_2_component_1"
    FROM "menura_2_filter"
    GROUP BY 1, 2, 3
  ),
  "menura_7_checked" AS (
    SELECT
      "menura_value"."dimension_1" AS "dimension_1",
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1",
      "menura_value"."metric_2_component_1" AS "metric_2_component_1"
    FROM "menura_6_values" AS "menura_value"
    LEFT JOIN "menura_5_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_8_snapshot" AS (
    SELECT
      CAST("dimension_1" AS Nullable(Date32)) AS "dimension_1",
      accurateCast("entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast(AVG("metric_1_component_1"), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(AVG("metric_2_component_1"), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_7_checked"
    GROUP BY 1, 2
  ),
  "menura_9_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("metric_2_component_1")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_8_snapshot"
    GROUP BY 1
  ),
  "menura_10_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST(CAST(addMonths("dimension_1", toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)) AS Nullable(Date32)) AS "context_1_shifted_time",
      CAST(TRUE AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_9_aggregate"
  ),
  "menura_11_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_12_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD"
    FROM "menura_11_scan"
    WHERE (("PERIOD" >= CAST(addMonths(toDate32(toStartOfMonth({p1:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32))) AND ("PERIOD" < CAST(addMonths(CAST(addMonths(toDate32(toStartOfMonth({p2:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32))))
  ),
  "menura_13_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_14_periods" AS (
    SELECT
      "menura_start" AS "period"
    FROM (SELECT arrayJoin(arrayMap(menura_offset -> addMonths(assumeNotNull(toDate32(toStartOfMonth(CAST(addMonths(toDate32(toStartOfMonth({p1:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32))))), menura_offset), range(toUInt64(if(ifNull(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth(CAST(addMonths(toDate32(toStartOfMonth({p1:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32))))), CAST(addDays(CAST(addMonths(CAST(addMonths(toDate32(toStartOfMonth({p2:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32))) < 0, 1), 0, intDiv(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth(CAST(addMonths(toDate32(toStartOfMonth({p1:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32))))), CAST(addDays(CAST(addMonths(CAST(addMonths(toDate32(toStartOfMonth({p2:Date32})), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32)), toInt64(toInt64(1))) AS Nullable(Date32)), toInt64(accurateCast(-toInt256(toInt64(1)), 'Nullable(Int128)'))) AS Nullable(Date32))), 1) + 1))))) AS "menura_start") AS "menura_periods"
  ),
  "menura_15_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_13_scan" AS "menura_row"
    CROSS JOIN "menura_14_periods" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_16_values" AS (
    SELECT
      toDate32(toStartOfMonth("PERIOD")) AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1"
    FROM "menura_12_filter"
    GROUP BY 1, 2, 3
  ),
  "menura_17_checked" AS (
    SELECT
      "menura_value"."dimension_1" AS "dimension_1",
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_16_values" AS "menura_value"
    LEFT JOIN "menura_15_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_18_snapshot" AS (
    SELECT
      CAST("dimension_1" AS Nullable(Date32)) AS "dimension_1",
      accurateCast("entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast(AVG("metric_1_component_1"), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_17_checked"
    GROUP BY 1, 2
  ),
  "menura_19_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_18_snapshot"
    GROUP BY 1
  ),
  "menura_20_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS Nullable(Bool)) AS "context_1_time_present"
    FROM "menura_19_aggregate"
  ),
  "menura_21_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_1_time_present",
      TRUE AS "menura.present"
    FROM "menura_20_project"
  ),
  "menura_22_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_10_project" AS "menura_result"
    LEFT JOIN "menura_21_present" AS "menura_context" ON isNotDistinctFrom("menura_result"."context_1_shifted_time", "menura_context"."dimension_1") AND isNotDistinctFrom("menura_result"."context_1_time_present", "menura_context"."context_1_time_present")
  ),
  "menura_23_project" AS (
    SELECT
      "dimension_1" AS "period",
      "metric_1_component_1" AS "average_balance",
      "context_1_component_1" AS "average_balance_prior_month"
    FROM "menura_22_lookup"
  )
SELECT
  "period",
  "average_balance",
  "average_balance_prior_month"
FROM "menura_23_project"
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