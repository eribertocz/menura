WITH
  "menura_1_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "region" AS "REGION",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_3_filter" AS (
    SELECT
      "CUSTOMER_ID",
      "REGION",
      "VALID_FROM",
      "VALID_TO"
    FROM "menura_2_scan"
    WHERE (CASE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END) AND TRUE
  ),
  "menura_4_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_5_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("PERIOD")) AS "period"
    FROM "menura_4_scan"
    GROUP BY 1
  ),
  "menura_6_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_3_filter" AS "menura_row"
    CROSS JOIN "menura_5_aggregate" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_7_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "region" AS "REGION",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_8_filter" AS (
    SELECT
      "CUSTOMER_ID",
      "REGION",
      "VALID_FROM",
      "VALID_TO"
    FROM "menura_7_scan"
    WHERE (CASE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END) AND TRUE
  ),
  "menura_9_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_10_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("PERIOD")) AS "period"
    FROM "menura_9_scan"
    GROUP BY 1
  ),
  "menura_11_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_8_filter" AS "menura_row"
    CROSS JOIN "menura_10_aggregate" AS "menura_period"
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
  "menura_13_values" AS (
    SELECT
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      minOrNull("BALANCE") AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1, 2
  ),
  "menura_14_checked" AS (
    SELECT
      "menura_value"."entity_1",
      "menura_value"."menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_13_values" AS "menura_value"
    LEFT JOIN "menura_6_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
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
    FROM "menura_6_population" AS "menura_expected"
    LEFT JOIN "menura_15_present" AS "menura_present" ON "menura_present"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.period" = "menura_expected"."period"
    WHERE "menura_present"."entity_1" IS NULL
  ),
  "menura_17_added" AS (
    SELECT
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period"
    FROM "menura_12_project"
  ),
  "menura_18_zero" AS (
    SELECT
      "menura_fill"."entity_1",
      "menura_fill"."menura.period",
      accurateCast(0, 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_17_added" AS "menura_fill"
    JOIN "menura_16_missing" AS "menura_missing" ON "menura_fill"."entity_1" = "menura_missing"."entity_1" AND "menura_fill"."menura.period" = "menura_missing"."menura.period"
  ),
  "menura_19_candidates" AS (
    SELECT
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_14_checked"
    UNION ALL
    SELECT
      "entity_1",
      "menura.period",
      "metric_1_component_1"
    FROM "menura_18_zero"
  ),
  "menura_20_selected" AS (
    SELECT
      MAX("menura.period") AS "menura.period"
    FROM "menura_13_values"
  ),
  "menura_21_snapshot" AS (
    SELECT
      "menura_candidate"."entity_1",
      accurateCast("menura_candidate"."metric_1_component_1", 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_19_candidates" AS "menura_candidate"
    JOIN "menura_20_selected" AS "menura_selected" ON "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  ),
  "menura_22_aggregate" AS (
    SELECT
      accurateCast(minOrNull("metric_1_component_1"), 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_21_snapshot"
  ),
  "menura_23_project" AS (
    SELECT
      "metric_1_component_1" AS "visible_zero_minimum"
    FROM "menura_22_aggregate"
  )
SELECT
  "visible_zero_minimum"
FROM "menura_23_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1