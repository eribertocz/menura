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
    FROM "shop"."stamp_ranges"
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
    WHERE (toDate32("menura_row"."VALID_FROM", {p3:String}) <= addDays(addMonths("menura_period"."period", 1), -1) AND (nullIf("menura_row"."VALID_TO", toDateTime64('2000-01-01 00:00:00', 6, 'UTC')) IS NULL OR nullIf("menura_row"."VALID_TO", toDateTime64('2000-01-01 00:00:00', 6, 'UTC')) > toTimeZone(toDateTime64("menura_period"."period", 6, {p3:String}), 'UTC'))) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_6_values" AS (
    SELECT
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1, 2
  ),
  "menura_7_checked" AS (
    SELECT
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_6_values" AS "menura_value"
    LEFT JOIN "menura_5_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_8_present" AS (
    SELECT
      "entity_1",
      "menura.period"
    FROM "menura_7_checked"
    GROUP BY 1, 2
  ),
  "menura_9_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "entity_1",
      "menura_expected"."period" AS "menura.period"
    FROM "menura_5_population" AS "menura_expected"
    LEFT JOIN "menura_8_present" AS "menura_present" ON "menura_present"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.period" = "menura_expected"."period"
    WHERE "menura_present"."entity_1" IS NULL
  ),
  "menura_10_selected" AS (
    SELECT
      MAX("menura.period") AS "menura.period"
    FROM "menura_6_values"
  ),
  "menura_11_violations" AS (
    SELECT
      'RADD-01: missing: error and an expected entity has no row in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE' AS "menura.violation"
    FROM "menura_9_missing"
  ),
  "menura_12_snapshot" AS (
    SELECT
      accurateCast("menura_candidate"."entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast("menura_candidate"."metric_1_component_1", 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_7_checked" AS "menura_candidate"
    JOIN "menura_10_selected" AS "menura_selected" ON "menura_candidate"."menura.period" = "menura_selected"."menura.period"
    UNION ALL
    SELECT
      accurateCast(NULL, 'Nullable(Int32)') AS "entity_1",
      accurateCast(nullIf(throwIf(ifNull(min("menura.violation") = 'RADD-01: missing: error and an expected entity has no row in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE', 0), 'RADD-01: missing: error and an expected entity has no row in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE') + throwIf(ifNull(min("menura.violation") = 'RADD-02: unexpected: error and an unexpected row exists in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE', 0), 'RADD-02: unexpected: error and an unexpected row exists in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE') + throwIf(ifNull(min("menura.violation") = 'RADD-03: nulls: error and a NULL value exists in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE', 0), 'RADD-03: nulls: error and a NULL value exists in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE') + throwIf(ifNull(min("menura.violation") = 'RADD-04: on_stale: error and a carried value is too old in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE', 0), 'RADD-04: on_stale: error and a carried value is too old in metric STAMP_EXCLUSIVE_OPEN_END_BALANCE'), 0), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_11_violations"
    HAVING min("menura.violation") IS NOT NULL
  ),
  "menura_13_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_12_snapshot"
  ),
  "menura_14_project" AS (
    SELECT
      "metric_1_component_1" AS "stamp_exclusive_open_end_balance"
    FROM "menura_13_aggregate"
  )
SELECT
  "stamp_exclusive_open_end_balance"
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