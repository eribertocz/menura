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
  "menura_6_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_7_periods" AS (
    SELECT
      "menura_start" AS "period"
    FROM (SELECT arrayJoin(arrayMap(menura_offset -> addMonths(assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), menura_offset), range(toUInt64(if(ifNull(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), {p2:Date32}) < 0, 1), 0, intDiv(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), {p2:Date32}), 1) + 1))))) AS "menura_start") AS "menura_periods"
  ),
  "menura_8_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_6_scan" AS "menura_row"
    CROSS JOIN "menura_7_periods" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_9_project" AS (
    SELECT
      accurateCast(NULL, 'Nullable(Int32)') AS "BALANCE",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "menura_8_population"
  ),
  "menura_10_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD"
    FROM "menura_9_project"
    WHERE ("PERIOD" >= {p1:Date32}) AND ("PERIOD" <= {p2:Date32})
  ),
  "menura_11_values" AS (
    SELECT
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1, 2
  ),
  "menura_12_checked" AS (
    SELECT
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_11_values" AS "menura_value"
    LEFT JOIN "menura_5_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_13_present" AS (
    SELECT
      "entity_1",
      "menura.period"
    FROM "menura_12_checked"
    GROUP BY 1, 2
  ),
  "menura_14_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "entity_1",
      "menura_expected"."period" AS "menura.period"
    FROM "menura_5_population" AS "menura_expected"
    LEFT JOIN "menura_13_present" AS "menura_present" ON "menura_present"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.period" = "menura_expected"."period"
    WHERE "menura_present"."entity_1" IS NULL
  ),
  "menura_15_added" AS (
    SELECT
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period"
    FROM "menura_10_filter"
  ),
  "menura_16_zero" AS (
    SELECT
      "menura_fill"."entity_1" AS "entity_1",
      "menura_fill"."menura.period" AS "menura.period",
      accurateCast(0, 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_15_added" AS "menura_fill"
    JOIN "menura_14_missing" AS "menura_missing" ON "menura_fill"."entity_1" = "menura_missing"."entity_1" AND "menura_fill"."menura.period" = "menura_missing"."menura.period"
  ),
  "menura_17_candidates" AS (
    SELECT
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_12_checked"
    UNION ALL
    SELECT
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_16_zero"
  ),
  "menura_18_snapshot" AS (
    SELECT
      accurateCast("entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast(AVG("metric_1_component_1"), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_17_candidates"
    GROUP BY 1
  ),
  "menura_19_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_18_snapshot"
  ),
  "menura_20_project" AS (
    SELECT
      "metric_1_component_1" AS "average_with_zero"
    FROM "menura_19_aggregate"
  )
SELECT
  "average_with_zero"
FROM "menura_20_project"
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