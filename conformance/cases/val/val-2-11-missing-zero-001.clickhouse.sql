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
      "customer_id" AS "BALANCE_SEGMENT.CUSTOMER_ID",
      "segment" AS "BALANCE_SEGMENT.SEGMENT",
      "segment_from" AS "BALANCE_SEGMENT.SEGMENT_FROM",
      "segment_to" AS "BALANCE_SEGMENT.SEGMENT_TO"
    FROM "shop"."segments"
  ),
  "menura_3_rows" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      ROW_NUMBER() OVER () AS "menura.row"
    FROM "menura_1_scan"
  ),
  "menura_4_versions" AS (
    SELECT
      "menura_row"."BALANCE" AS "BALANCE",
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_row"."PERIOD" AS "PERIOD",
      "menura_row"."menura.row_identity" AS "menura.row_identity",
      "menura_row"."menura.row" AS "menura.row",
      "menura_version"."BALANCE_SEGMENT.CUSTOMER_ID" AS "BALANCE_SEGMENT.CUSTOMER_ID",
      "menura_version"."BALANCE_SEGMENT.SEGMENT" AS "BALANCE_SEGMENT.SEGMENT",
      "menura_version"."BALANCE_SEGMENT.SEGMENT_FROM" AS "BALANCE_SEGMENT.SEGMENT_FROM",
      "menura_version"."BALANCE_SEGMENT.SEGMENT_TO" AS "BALANCE_SEGMENT.SEGMENT_TO",
      CASE WHEN "menura_version"."BALANCE_SEGMENT.SEGMENT_FROM" IS NOT NULL AND "menura_version"."BALANCE_SEGMENT.SEGMENT_FROM" <= "menura_row"."PERIOD" AND ("menura_version"."BALANCE_SEGMENT.SEGMENT_TO" IS NULL OR "menura_row"."PERIOD" < "menura_version"."BALANCE_SEGMENT.SEGMENT_TO") THEN 1 ELSE 0 END AS "menura.covers"
    FROM "menura_3_rows" AS "menura_row"
    LEFT JOIN "menura_2_scan" AS "menura_version" ON "menura_row"."CUSTOMER_ID" = "menura_version"."BALANCE_SEGMENT.CUSTOMER_ID"
  ),
  "menura_5_covered" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCE_SEGMENT.CUSTOMER_ID",
      "BALANCE_SEGMENT.SEGMENT",
      "BALANCE_SEGMENT.SEGMENT_FROM",
      "BALANCE_SEGMENT.SEGMENT_TO",
      "menura.covers",
      MAX("menura.covers") OVER (PARTITION BY "menura.row") AS "menura.covered",
      ROW_NUMBER() OVER (PARTITION BY "menura.row") AS "menura.rank"
    FROM "menura_4_versions"
  ),
  "menura_6_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      CASE WHEN "menura.covers" = 1 THEN "BALANCE_SEGMENT.CUSTOMER_ID" ELSE NULL END AS "BALANCE_SEGMENT.CUSTOMER_ID",
      CASE WHEN "menura.covers" = 1 THEN "BALANCE_SEGMENT.SEGMENT" ELSE NULL END AS "BALANCE_SEGMENT.SEGMENT",
      CASE WHEN "menura.covers" = 1 THEN "BALANCE_SEGMENT.SEGMENT_FROM" ELSE NULL END AS "BALANCE_SEGMENT.SEGMENT_FROM",
      CASE WHEN "menura.covers" = 1 THEN "BALANCE_SEGMENT.SEGMENT_TO" ELSE NULL END AS "BALANCE_SEGMENT.SEGMENT_TO"
    FROM "menura_5_covered"
    WHERE "menura.covers" = 1 OR ("menura.covered" = 0 AND "menura.rank" = 1)
  ),
  "menura_7_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCE_SEGMENT.CUSTOMER_ID",
      "BALANCE_SEGMENT.SEGMENT",
      "BALANCE_SEGMENT.SEGMENT_FROM",
      "BALANCE_SEGMENT.SEGMENT_TO"
    FROM "menura_6_join"
    WHERE ("PERIOD" >= {p1:Date32}) AND ("PERIOD" <= {p2:Date32})
  ),
  "menura_8_deduplicate" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCE_SEGMENT.CUSTOMER_ID",
      "BALANCE_SEGMENT.SEGMENT",
      "BALANCE_SEGMENT.SEGMENT_FROM",
      "BALANCE_SEGMENT.SEGMENT_TO"
    FROM "menura_7_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCE_SEGMENT.SEGMENT", "menura.row_identity") = 1
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
  "menura_15_project" AS (
    SELECT
      accurateCast(NULL, 'Nullable(Int32)') AS "BALANCE",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "menura_14_population"
  ),
  "menura_16_scan" AS (
    SELECT
      "customer_id" AS "BALANCE_SEGMENT.CUSTOMER_ID",
      "segment" AS "BALANCE_SEGMENT.SEGMENT",
      "segment_from" AS "BALANCE_SEGMENT.SEGMENT_FROM",
      "segment_to" AS "BALANCE_SEGMENT.SEGMENT_TO"
    FROM "shop"."segments"
  ),
  "menura_17_rows" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      ROW_NUMBER() OVER () AS "menura.row"
    FROM "menura_15_project"
  ),
  "menura_18_versions" AS (
    SELECT
      "menura_row"."BALANCE" AS "BALANCE",
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_row"."PERIOD" AS "PERIOD",
      "menura_row"."menura.row" AS "menura.row",
      "menura_version"."BALANCE_SEGMENT.CUSTOMER_ID" AS "BALANCE_SEGMENT.CUSTOMER_ID",
      "menura_version"."BALANCE_SEGMENT.SEGMENT" AS "BALANCE_SEGMENT.SEGMENT",
      "menura_version"."BALANCE_SEGMENT.SEGMENT_FROM" AS "BALANCE_SEGMENT.SEGMENT_FROM",
      "menura_version"."BALANCE_SEGMENT.SEGMENT_TO" AS "BALANCE_SEGMENT.SEGMENT_TO",
      CASE WHEN "menura_version"."BALANCE_SEGMENT.SEGMENT_FROM" IS NOT NULL AND "menura_version"."BALANCE_SEGMENT.SEGMENT_FROM" <= "menura_row"."PERIOD" AND ("menura_version"."BALANCE_SEGMENT.SEGMENT_TO" IS NULL OR "menura_row"."PERIOD" < "menura_version"."BALANCE_SEGMENT.SEGMENT_TO") THEN 1 ELSE 0 END AS "menura.covers"
    FROM "menura_17_rows" AS "menura_row"
    LEFT JOIN "menura_16_scan" AS "menura_version" ON "menura_row"."CUSTOMER_ID" = "menura_version"."BALANCE_SEGMENT.CUSTOMER_ID"
  ),
  "menura_19_covered" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "BALANCE_SEGMENT.CUSTOMER_ID",
      "BALANCE_SEGMENT.SEGMENT",
      "BALANCE_SEGMENT.SEGMENT_FROM",
      "BALANCE_SEGMENT.SEGMENT_TO",
      "menura.covers",
      MAX("menura.covers") OVER (PARTITION BY "menura.row") AS "menura.covered",
      ROW_NUMBER() OVER (PARTITION BY "menura.row") AS "menura.rank"
    FROM "menura_18_versions"
  ),
  "menura_20_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      CASE WHEN "menura.covers" = 1 THEN "BALANCE_SEGMENT.CUSTOMER_ID" ELSE NULL END AS "BALANCE_SEGMENT.CUSTOMER_ID",
      CASE WHEN "menura.covers" = 1 THEN "BALANCE_SEGMENT.SEGMENT" ELSE NULL END AS "BALANCE_SEGMENT.SEGMENT",
      CASE WHEN "menura.covers" = 1 THEN "BALANCE_SEGMENT.SEGMENT_FROM" ELSE NULL END AS "BALANCE_SEGMENT.SEGMENT_FROM",
      CASE WHEN "menura.covers" = 1 THEN "BALANCE_SEGMENT.SEGMENT_TO" ELSE NULL END AS "BALANCE_SEGMENT.SEGMENT_TO"
    FROM "menura_19_covered"
    WHERE "menura.covers" = 1 OR ("menura.covered" = 0 AND "menura.rank" = 1)
  ),
  "menura_21_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "BALANCE_SEGMENT.CUSTOMER_ID",
      "BALANCE_SEGMENT.SEGMENT",
      "BALANCE_SEGMENT.SEGMENT_FROM",
      "BALANCE_SEGMENT.SEGMENT_TO"
    FROM "menura_20_join"
    WHERE ("PERIOD" >= {p1:Date32}) AND ("PERIOD" <= {p2:Date32})
  ),
  "menura_22_values" AS (
    SELECT
      "BALANCE_SEGMENT.SEGMENT" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      minOrNull("BALANCE") AS "metric_1_component_1"
    FROM "menura_8_deduplicate"
    GROUP BY 1, 2, 3
  ),
  "menura_23_checked" AS (
    SELECT
      "menura_value"."dimension_1" AS "dimension_1",
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_22_values" AS "menura_value"
    LEFT JOIN "menura_11_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_24_present" AS (
    SELECT
      "entity_1",
      "menura.period"
    FROM "menura_23_checked"
    GROUP BY 1, 2
  ),
  "menura_25_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "entity_1",
      "menura_expected"."period" AS "menura.period"
    FROM "menura_11_population" AS "menura_expected"
    LEFT JOIN "menura_24_present" AS "menura_present" ON "menura_present"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.period" = "menura_expected"."period"
    WHERE "menura_present"."entity_1" IS NULL
  ),
  "menura_26_added" AS (
    SELECT
      "BALANCE_SEGMENT.SEGMENT" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period"
    FROM "menura_21_filter"
  ),
  "menura_27_zero" AS (
    SELECT
      "menura_fill"."dimension_1" AS "dimension_1",
      "menura_fill"."entity_1" AS "entity_1",
      "menura_fill"."menura.period" AS "menura.period",
      accurateCast(0, 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_26_added" AS "menura_fill"
    JOIN "menura_25_missing" AS "menura_missing" ON "menura_fill"."entity_1" = "menura_missing"."entity_1" AND "menura_fill"."menura.period" = "menura_missing"."menura.period"
  ),
  "menura_28_candidates" AS (
    SELECT
      "dimension_1",
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_23_checked"
    UNION ALL
    SELECT
      "dimension_1",
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_27_zero"
  ),
  "menura_29_selected" AS (
    SELECT
      "dimension_1",
      MAX("menura.period") AS "menura.period"
    FROM "menura_22_values"
    GROUP BY 1
  ),
  "menura_30_snapshot" AS (
    SELECT
      CAST("menura_candidate"."dimension_1" AS Nullable(String)) AS "dimension_1",
      accurateCast("menura_candidate"."entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast("menura_candidate"."metric_1_component_1", 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_28_candidates" AS "menura_candidate"
    JOIN "menura_29_selected" AS "menura_selected" ON isNotDistinctFrom("menura_candidate"."dimension_1", "menura_selected"."dimension_1") AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  ),
  "menura_31_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      accurateCast(minOrNull("metric_1_component_1"), 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_30_snapshot"
    GROUP BY 1
  ),
  "menura_32_project" AS (
    SELECT
      "dimension_1" AS "segment",
      "metric_1_component_1" AS "lowest_balance"
    FROM "menura_31_aggregate"
  )
SELECT
  "segment",
  "lowest_balance"
FROM "menura_32_project"
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