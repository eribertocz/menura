WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "paid" AS "PAID",
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
      "CUSTOMER_ID",
      "PAID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_4_filter" AS (
    SELECT
      "CUSTOMER_ID",
      "PAID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_3_join"
    WHERE ("PERIOD" >= {p1:Date32}) AND ("PERIOD" <= {p2:Date32})
  ),
  "menura_5_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "PAID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_4_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_6_aggregate" AS (
    SELECT
      "BALANCES_CUSTOMERS.CATEGORY" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("PAID")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_5_deduplicate"
    GROUP BY 1
  ),
  "menura_7_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."balances"
  ),
  "menura_8_scan" AS (
    SELECT
      "category" AS "BALANCES_CUSTOMERS.CATEGORY",
      "customer_id" AS "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_9_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_7_scan"
    LEFT JOIN "menura_8_scan" ON "menura_7_scan"."CUSTOMER_ID" = "menura_8_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_10_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_9_join"
    WHERE ("PERIOD" >= {p1:Date32}) AND ("PERIOD" <= {p2:Date32})
  ),
  "menura_11_deduplicate" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_10_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_12_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_13_periods" AS (
    SELECT
      "menura_start" AS "period"
    FROM (SELECT arrayJoin(arrayMap(menura_offset -> addMonths(assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), menura_offset), range(toUInt64(if(ifNull(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), {p2:Date32}) < 0, 1), 0, intDiv(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth({p1:Date32}))), {p2:Date32}), 1) + 1))))) AS "menura_start") AS "menura_periods"
  ),
  "menura_14_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_12_scan" AS "menura_row"
    CROSS JOIN "menura_13_periods" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_15_values" AS (
    SELECT
      "BALANCES_CUSTOMERS.CATEGORY" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1"
    FROM "menura_11_deduplicate"
    GROUP BY 1, 2, 3
  ),
  "menura_16_checked" AS (
    SELECT
      "menura_value"."dimension_1" AS "dimension_1",
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_15_values" AS "menura_value"
    LEFT JOIN "menura_14_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_17_selected" AS (
    SELECT
      "dimension_1",
      MAX("menura.period") AS "menura.period"
    FROM "menura_15_values"
    GROUP BY 1
  ),
  "menura_18_snapshot" AS (
    SELECT
      CAST("menura_candidate"."dimension_1" AS Nullable(String)) AS "dimension_1",
      accurateCast("menura_candidate"."entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast("menura_candidate"."metric_1_component_1", 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_16_checked" AS "menura_candidate"
    JOIN "menura_17_selected" AS "menura_selected" ON isNotDistinctFrom("menura_candidate"."dimension_1", "menura_selected"."dimension_1") AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  ),
  "menura_19_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_18_snapshot"
    GROUP BY 1
  ),
  "menura_20_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_6_aggregate"
  ),
  "menura_21_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_19_aggregate"
  ),
  "menura_22_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_6_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_19_aggregate"
  ),
  "menura_23_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1"
    FROM "menura_22_keys" AS "menura_keys"
    LEFT JOIN "menura_20_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_21_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_24_project" AS (
    SELECT
      "dimension_1" AS "category",
      "metric_1_component_1" AS "last_global",
      "metric_2_component_1" AS "total_paid"
    FROM "menura_23_stitch"
  )
SELECT
  "category",
  "last_global",
  "total_paid"
FROM "menura_24_project"
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