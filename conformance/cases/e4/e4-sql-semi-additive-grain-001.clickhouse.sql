WITH
  "menura_1_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_2_project" AS (
    SELECT
      "BALANCE" AS "BALANCE",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      "PERIOD" AS "PERIOD",
      CAST(toDate32(toStartOfYear("PERIOD")) AS Nullable(Date32)) AS "menura.dimension_1"
    FROM "menura_1_scan"
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
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
      CAST(toDate32(toStartOfYear("PERIOD")) AS Nullable(Date32)) AS "menura.dimension_1"
    FROM "menura_4_scan"
  ),
  "menura_6_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("PERIOD")) AS "period"
    FROM "menura_5_project"
    GROUP BY 1
  ),
  "menura_7_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_3_scan" AS "menura_row"
    CROSS JOIN "menura_6_aggregate" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_8_values" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_1_component_1"
    FROM "menura_2_project"
    GROUP BY 1, 2, 3
  ),
  "menura_9_checked" AS (
    SELECT
      "menura_value"."dimension_1",
      "menura_value"."entity_1",
      "menura_value"."menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_8_values" AS "menura_value"
    LEFT JOIN "menura_7_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_10_selected" AS (
    SELECT
      "dimension_1",
      MAX("menura.period") AS "menura.period"
    FROM "menura_8_values"
    GROUP BY 1
  ),
  "menura_11_snapshot" AS (
    SELECT
      "menura_candidate"."dimension_1",
      "menura_candidate"."entity_1",
      accurateCast("menura_candidate"."metric_1_component_1", 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_9_checked" AS "menura_candidate"
    JOIN "menura_10_selected" AS "menura_selected" ON isNotDistinctFrom("menura_candidate"."dimension_1", "menura_selected"."dimension_1") AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  ),
  "menura_12_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_11_snapshot"
    GROUP BY 1
  ),
  "menura_13_project" AS (
    SELECT
      "dimension_1" AS "period_year",
      "metric_1_component_1" AS "balance"
    FROM "menura_12_aggregate"
  ),
  "menura_14_sort" AS (
    SELECT
      "period_year",
      "balance"
    FROM "menura_13_project"
    ORDER BY "period_year" ASC NULLS LAST
  )
SELECT
  "period_year",
  "balance"
FROM "menura_14_sort"
ORDER BY "period_year" ASC NULLS LAST
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1