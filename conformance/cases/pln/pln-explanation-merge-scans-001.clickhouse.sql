WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "paid" AS "PAID",
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
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "PAID",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "BALANCES_CUSTOMERS.CATEGORY" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("PAID")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(coalesce(count("PAID"), 0), 'Nullable(Int64)') AS "metric_3_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."balances"
  ),
  "menura_7_scan" AS (
    SELECT
      "category" AS "BALANCES_CUSTOMERS.CATEGORY",
      "customer_id" AS "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_8_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_6_scan"
    LEFT JOIN "menura_7_scan" ON "menura_6_scan"."CUSTOMER_ID" = "menura_7_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_9_deduplicate" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_8_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_10_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_11_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."balances"
  ),
  "menura_12_scan" AS (
    SELECT
      "category" AS "BALANCES_CUSTOMERS.CATEGORY",
      "customer_id" AS "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_13_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_11_scan"
    LEFT JOIN "menura_12_scan" ON "menura_11_scan"."CUSTOMER_ID" = "menura_12_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_14_deduplicate" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_13_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_15_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("PERIOD")) AS "period"
    FROM "menura_14_deduplicate"
    GROUP BY 1
  ),
  "menura_16_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_10_scan" AS "menura_row"
    CROSS JOIN "menura_15_aggregate" AS "menura_period"
    WHERE ("menura_row"."VALID_FROM" <= addDays(addMonths("menura_period"."period", 1), -1) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")) AND "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_17_values" AS (
    SELECT
      "BALANCES_CUSTOMERS.CATEGORY" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      toDate32(toStartOfMonth("PERIOD")) AS "menura.period",
      sumOrNull(toInt256("BALANCE")) AS "metric_2_component_1"
    FROM "menura_9_deduplicate"
    GROUP BY 1, 2, 3
  ),
  "menura_18_checked" AS (
    SELECT
      "menura_value"."dimension_1" AS "dimension_1",
      "menura_value"."entity_1" AS "entity_1",
      "menura_value"."menura.period" AS "menura.period",
      "menura_value"."metric_2_component_1" AS "metric_2_component_1"
    FROM "menura_17_values" AS "menura_value"
    LEFT JOIN "menura_16_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_19_selected" AS (
    SELECT
      "dimension_1",
      MAX("menura.period") AS "menura.period"
    FROM "menura_17_values"
    GROUP BY 1
  ),
  "menura_20_snapshot" AS (
    SELECT
      CAST("menura_candidate"."dimension_1" AS Nullable(String)) AS "dimension_1",
      accurateCast("menura_candidate"."entity_1", 'Nullable(Int32)') AS "entity_1",
      accurateCast("menura_candidate"."metric_2_component_1", 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_18_checked" AS "menura_candidate"
    JOIN "menura_19_selected" AS "menura_selected" ON isNotDistinctFrom("menura_candidate"."dimension_1", "menura_selected"."dimension_1") AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  ),
  "menura_21_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("metric_2_component_1")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_20_snapshot"
    GROUP BY 1
  ),
  "menura_22_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "metric_3_component_1",
      TRUE AS "menura.present"
    FROM "menura_5_aggregate"
  ),
  "menura_23_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_21_aggregate"
  ),
  "menura_24_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_5_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_21_aggregate"
  ),
  "menura_25_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_3_component_1" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "metric_3_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1"
    FROM "menura_24_keys" AS "menura_keys"
    LEFT JOIN "menura_22_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_23_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_26_project" AS (
    SELECT
      "dimension_1" AS "category",
      "metric_1_component_1" AS "total_paid",
      "metric_2_component_1" AS "last_global",
      "metric_3_component_1" AS "paid_rows"
    FROM "menura_25_stitch"
  )
SELECT
  "category",
  "total_paid",
  "last_global",
  "paid_rows"
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