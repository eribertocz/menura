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
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1, 2
  ),
  "menura_7_checked" AS (
    SELECT
      "menura_value"."entity_1",
      "menura_value"."menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_6_values" AS "menura_value"
    LEFT JOIN "menura_5_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_8_snapshot" AS (
    SELECT
      "entity_1",
      accurateCast("metric_1_component_1", 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_7_checked"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "entity_1" ORDER BY "menura.period" ASC NULLS LAST) = 1
  ),
  "menura_9_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_8_snapshot"
  ),
  "menura_10_project" AS (
    SELECT
      "metric_1_component_1" AS "first_per_entity"
    FROM "menura_9_aggregate"
  )
SELECT
  "first_per_entity"
FROM "menura_10_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1