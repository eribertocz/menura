WITH
  "menura_1_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."balances"
  ),
  "menura_2_scan" AS (
    SELECT
      "category" AS "BALANCES_CUSTOMERS.CATEGORY",
      "customer_id" AS "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_3_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_4_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_3_join"
    WHERE ("PERIOD" >= {p1:Date32}) AND ("PERIOD" <= {p2:Date32})
  ),
  "menura_5_deduplicate" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_4_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
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
  "menura_9_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_10_periods" AS (
    SELECT
      "menura_start" AS "period"
    FROM (SELECT arrayJoin(arrayMap(menura_offset -> addMonths(assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), menura_offset), range(toUInt64(if(ifNull(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), {p2:Date32}) < 0, 1), 0, intDiv(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), {p2:Date32}), 1) + 1))))) AS "menura_start") AS "menura_periods"
  ),
  "menura_11_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_9_scan" AS "menura_row"
    CROSS JOIN "menura_10_periods" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_12_project" AS (
    SELECT
      accurateCast(NULL, 'Nullable(Int32)') AS "BALANCE",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "menura_11_population"
  ),
  "menura_13_scan" AS (
    SELECT
      "category" AS "BALANCES_CUSTOMERS.CATEGORY",
      "customer_id" AS "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_14_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_12_project"
    LEFT JOIN "menura_13_scan" ON "menura_12_project"."CUSTOMER_ID" = "menura_13_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_15_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_14_join"
    WHERE ("PERIOD" >= {p1:Date32}) AND ("PERIOD" <= {p2:Date32})
  ),
  "menura_16_values" AS (
    SELECT
      "BALANCES_CUSTOMERS.CATEGORY" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      minOrNull("BALANCE") AS "metric_1_component_1"
    FROM "menura_5_deduplicate"
    GROUP BY 1, 2, 3
  ),
  "menura_17_checked" AS (
    SELECT
      "menura_value"."dimension_1" AS "dimension_1",
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_16_values" AS "menura_value"
    LEFT JOIN "menura_8_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_18_present" AS (
    SELECT
      "entity_1",
      "menura.period"
    FROM "menura_17_checked"
    GROUP BY 1, 2
  ),
  "menura_19_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "entity_1",
      "menura_expected"."period" AS "menura.period"
    FROM "menura_8_population" AS "menura_expected"
    LEFT JOIN "menura_18_present" AS "menura_present" ON "menura_present"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.period" = "menura_expected"."period"
    WHERE "menura_present"."entity_1" IS NULL
  ),
  "menura_20_added" AS (
    SELECT
      "BALANCES_CUSTOMERS.CATEGORY" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period"
    FROM "menura_15_filter"
  ),
  "menura_21_zero" AS (
    SELECT
      "menura_fill"."dimension_1" AS "dimension_1",
      "menura_fill"."entity_1" AS "entity_1",
      "menura_fill"."menura.period" AS "menura.period",
      accurateCast(0, 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_20_added" AS "menura_fill"
    JOIN "menura_19_missing" AS "menura_missing" ON "menura_fill"."entity_1" = "menura_missing"."entity_1" AND "menura_fill"."menura.period" = "menura_missing"."menura.period"
  ),
  "menura_22_candidates" AS (
    SELECT
      "dimension_1",
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_17_checked"
    UNION ALL
    SELECT
      "dimension_1",
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_21_zero"
  ),
  "menura_23_selected" AS (
    SELECT
      "dimension_1",
      MAX("menura.period") AS "menura.period"
    FROM "menura_16_values"
    GROUP BY 1
  ),
  "menura_24_snapshot" AS (
    SELECT
      CAST("menura_candidate"."dimension_1" AS Nullable(String)) AS "dimension_1",
      accurateCast("menura_candidate"."entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast("menura_candidate"."metric_1_component_1", 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_22_candidates" AS "menura_candidate"
    JOIN "menura_23_selected" AS "menura_selected" ON isNotDistinctFrom("menura_candidate"."dimension_1", "menura_selected"."dimension_1") AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  ),
  "menura_25_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      accurateCast(minOrNull("metric_1_component_1"), 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_24_snapshot"
    GROUP BY 1
  ),
  "menura_26_project" AS (
    SELECT
      "dimension_1" AS "category",
      "metric_1_component_1" AS "zero_minimum"
    FROM "menura_25_aggregate"
  )
SELECT
  "category",
  "zero_minimum"
FROM "menura_26_project"
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