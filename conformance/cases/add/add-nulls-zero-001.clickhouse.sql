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
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_3_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_4_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("PERIOD")) AS "period"
    FROM "menura_3_scan"
    GROUP BY 1
  ),
  "menura_5_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_2_scan" AS "menura_row"
    CROSS JOIN "menura_4_aggregate" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_6_values" AS (
    SELECT
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      minOrNull("BALANCE") AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1, 2
  ),
  "menura_7_checked" AS (
    SELECT
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      coalesce("menura_value"."metric_1_component_1", 0) AS "metric_1_component_1"
    FROM "menura_6_values" AS "menura_value"
    LEFT JOIN "menura_5_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_8_selected" AS (
    SELECT
      MAX("menura.period") AS "menura.period"
    FROM "menura_6_values"
  ),
  "menura_9_snapshot" AS (
    SELECT
      accurateCast("menura_candidate"."entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast("menura_candidate"."metric_1_component_1", 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_7_checked" AS "menura_candidate"
    JOIN "menura_8_selected" AS "menura_selected" ON "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  ),
  "menura_10_aggregate" AS (
    SELECT
      accurateCast(minOrNull("metric_1_component_1"), 'Nullable(Int32)') AS "metric_1_component_1"
    FROM "menura_9_snapshot"
  ),
  "menura_11_project" AS (
    SELECT
      "metric_1_component_1" AS "nulls_zero"
    FROM "menura_10_aggregate"
  )
SELECT
  "nulls_zero"
FROM "menura_11_project"
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