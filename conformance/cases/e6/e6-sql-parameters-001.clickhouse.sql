WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "paid" AS "PAID"
    FROM "shop"."balances"
  ),
  "menura_2_project" AS (
    SELECT
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      "PAID" AS "PAID",
      CAST(CASE WHEN ("CUSTOMER_ID" = {p1:Int64}) THEN {p2:String} ELSE {p3:String} END AS Nullable(String)) AS "menura.dimension_1"
    FROM "menura_1_scan"
  ),
  "menura_3_aggregate" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("PAID")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_2_project"
    GROUP BY 1
  ),
  "menura_4_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_5_project" AS (
    SELECT
      "BALANCE" AS "BALANCE",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      "PERIOD" AS "PERIOD",
      CAST(CASE WHEN ("CUSTOMER_ID" = {p1:Int64}) THEN {p2:String} ELSE {p3:String} END AS Nullable(String)) AS "menura.dimension_1"
    FROM "menura_4_scan"
  ),
  "menura_6_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_7_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_8_project" AS (
    SELECT
      "BALANCE" AS "BALANCE",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      "PERIOD" AS "PERIOD",
      CAST(CASE WHEN ("CUSTOMER_ID" = {p1:Int64}) THEN {p2:String} ELSE {p3:String} END AS Nullable(String)) AS "menura.dimension_1"
    FROM "menura_7_scan"
  ),
  "menura_9_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("PERIOD")) AS "period"
    FROM "menura_8_project"
    GROUP BY 1
  ),
  "menura_10_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_6_scan" AS "menura_row"
    CROSS JOIN "menura_9_aggregate" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_11_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_12_project" AS (
    SELECT
      "BALANCE" AS "BALANCE",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      "PERIOD" AS "PERIOD",
      CAST(CASE WHEN ("CUSTOMER_ID" = {p1:Int64}) THEN {p2:String} ELSE {p3:String} END AS Nullable(String)) AS "menura.dimension_1"
    FROM "menura_11_scan"
  ),
  "menura_13_values" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1"
    FROM "menura_5_project"
    GROUP BY 1, 2, 3
  ),
  "menura_14_checked" AS (
    SELECT
      "menura_value"."dimension_1",
      "menura_value"."entity_1",
      "menura_value"."menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_13_values" AS "menura_value"
    LEFT JOIN "menura_10_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_15_present" AS (
    SELECT
      "entity_1",
      "menura.period"
    FROM "menura_14_checked"
    GROUP BY 1, 2
  ),
  "menura_16_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "entity_1",
      "menura_expected"."period" AS "menura.period"
    FROM "menura_10_population" AS "menura_expected"
    LEFT JOIN "menura_15_present" AS "menura_present" ON "menura_present"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.period" = "menura_expected"."period"
    WHERE "menura_present"."entity_1" IS NULL
  ),
  "menura_17_carried" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1"
    FROM "menura_12_project"
    GROUP BY 1, 2, 3
  ),
  "menura_18_carry" AS (
    SELECT
      "menura_fill"."dimension_1",
      "menura_fill"."entity_1",
      "menura_missing"."menura.period",
      "menura_fill"."metric_1_component_1",
      dateDiff('month', "menura_fill"."menura.period", "menura_missing"."menura.period") > 4 AS "menura.stale"
    FROM "menura_16_missing" AS "menura_missing"
    JOIN "menura_17_carried" AS "menura_fill" ON "menura_fill"."entity_1" = "menura_missing"."entity_1"
    WHERE "menura_fill"."menura.period" < "menura_missing"."menura.period"
    QUALIFY RANK() OVER (PARTITION BY "menura_missing"."entity_1", "menura_missing"."menura.period" ORDER BY "menura_fill"."menura.period" DESC NULLS LAST) = 1
  ),
  "menura_19_candidates" AS (
    SELECT
      "dimension_1",
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_14_checked"
    UNION ALL
    SELECT
      "dimension_1",
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_18_carry"
  ),
  "menura_20_selected" AS (
    SELECT
      "dimension_1",
      MAX("menura.period") AS "menura.period"
    FROM "menura_13_values"
    GROUP BY 1
  ),
  "menura_21_violations" AS (
    SELECT
      'RADD-04: on_stale: error and a carried value is too old in metric CARRIED_BALANCE' AS "menura.violation"
    FROM "menura_18_carry"
    WHERE "menura.stale"
  ),
  "menura_22_snapshot" AS (
    SELECT
      "menura_candidate"."dimension_1",
      "menura_candidate"."entity_1",
      accurateCast("menura_candidate"."metric_1_component_1", 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_19_candidates" AS "menura_candidate"
    JOIN "menura_20_selected" AS "menura_selected" ON isNotDistinctFrom("menura_candidate"."dimension_1", "menura_selected"."dimension_1") AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
    UNION ALL
    SELECT
      NULL AS "dimension_1",
      NULL AS "entity_1",
      nullIf(throwIf(ifNull(min("menura.violation") = 'RADD-01: missing: error and an expected entity has no row in metric CARRIED_BALANCE', 0), 'RADD-01: missing: error and an expected entity has no row in metric CARRIED_BALANCE') + throwIf(ifNull(min("menura.violation") = 'RADD-02: unexpected: error and an unexpected row exists in metric CARRIED_BALANCE', 0), 'RADD-02: unexpected: error and an unexpected row exists in metric CARRIED_BALANCE') + throwIf(ifNull(min("menura.violation") = 'RADD-03: nulls: error and a NULL value exists in metric CARRIED_BALANCE', 0), 'RADD-03: nulls: error and a NULL value exists in metric CARRIED_BALANCE') + throwIf(ifNull(min("menura.violation") = 'RADD-04: on_stale: error and a carried value is too old in metric CARRIED_BALANCE', 0), 'RADD-04: on_stale: error and a carried value is too old in metric CARRIED_BALANCE'), 0) AS "metric_1_component_1"
    FROM "menura_21_violations"
    HAVING min("menura.violation") IS NOT NULL
  ),
  "menura_23_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_22_snapshot"
    GROUP BY 1
  ),
  "menura_24_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_3_aggregate"
  ),
  "menura_25_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_23_aggregate"
  ),
  "menura_26_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_3_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_23_aggregate"
  ),
  "menura_27_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1"
    FROM "menura_26_keys" AS "menura_keys"
    LEFT JOIN "menura_24_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_25_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_28_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_2_component_1" AS "metric_2_component_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2"
    FROM "menura_27_stitch"
  ),
  "menura_29_project" AS (
    SELECT
      "dimension_1" AS "bucket",
      accurateCast(accurateCast(toInt256("metric_1_component_1") + toInt256({p4:Int64}), 'Nullable(Int128)'), 'Nullable(Int128)') AS "carried",
      "metric_2" AS "paid"
    FROM "menura_28_project"
  ),
  "menura_30_sort" AS (
    SELECT
      "bucket",
      "carried",
      "paid"
    FROM "menura_29_project"
    ORDER BY "bucket" ASC NULLS LAST
  )
SELECT
  "bucket",
  "carried",
  "paid"
FROM "menura_30_sort"
ORDER BY "bucket" ASC NULLS LAST
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1