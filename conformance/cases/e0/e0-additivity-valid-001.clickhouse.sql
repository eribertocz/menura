WITH
  "menura_1_scan" AS (
    SELECT
      "category" AS "CATEGORY",
      "customer_id" AS "CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "CATEGORY" AS "dimension_1",
      accurateCast(coalesce(count("CUSTOMER_ID"), 0), 'Nullable(Int64)') AS "metric_2_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."debt"
  ),
  "menura_4_scan" AS (
    SELECT
      "category" AS "DEBT_CUSTOMERS.CATEGORY",
      "customer_id" AS "DEBT_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."customers"
  ),
  "menura_5_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "menura.row_identity",
      "DEBT_CUSTOMERS.CATEGORY",
      "DEBT_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_3_scan"
    LEFT JOIN "menura_4_scan" ON "menura_3_scan"."CUSTOMER_ID" = "menura_4_scan"."DEBT_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_6_deduplicate" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "menura.row_identity",
      "DEBT_CUSTOMERS.CATEGORY",
      "DEBT_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_5_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "DEBT_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_7_aggregate" AS (
    SELECT
      "DEBT_CUSTOMERS.CATEGORY" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("BALANCE")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_6_deduplicate"
    GROUP BY 1
  ),
  "menura_8_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_2_aggregate"
  ),
  "menura_9_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_7_aggregate"
  ),
  "menura_10_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_2_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_7_aggregate"
  ),
  "menura_11_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1"
    FROM "menura_10_keys" AS "menura_keys"
    LEFT JOIN "menura_8_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_9_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_12_project" AS (
    SELECT
      "dimension_1" AS "category",
      "metric_1_component_1" AS "strict_balance",
      "metric_2_component_1" AS "customer_count"
    FROM "menura_11_stitch"
  )
SELECT
  "category",
  "strict_balance",
  "customer_count"
FROM "menura_12_project"
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