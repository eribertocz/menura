WITH
  "menura_1_scan" AS (
    SELECT
      "budget_id" AS "BUDGET_ID",
      "region_id" AS "REGION_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."budgets"
  ),
  "menura_2_scan" AS (
    SELECT
      "name" AS "BUDGETS_REGIONS.NAME",
      "region_id" AS "BUDGETS_REGIONS.REGION_ID"
    FROM "shop"."regions"
  ),
  "menura_3_join" AS (
    SELECT
      "BUDGET_ID",
      "REGION_ID",
      "menura.row_identity",
      "BUDGETS_REGIONS.NAME",
      "BUDGETS_REGIONS.REGION_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."REGION_ID" = "menura_2_scan"."BUDGETS_REGIONS.REGION_ID"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "BUDGET_ID",
      "REGION_ID",
      "menura.row_identity",
      "BUDGETS_REGIONS.NAME",
      "BUDGETS_REGIONS.REGION_ID"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BUDGETS_REGIONS.NAME", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "BUDGETS_REGIONS.NAME" AS "dimension_1",
      accurateCast(coalesce(count("BUDGET_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "region_id" AS "REGION_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."sales"
  ),
  "menura_7_scan" AS (
    SELECT
      "name" AS "SALES_REGIONS.NAME",
      "region_id" AS "SALES_REGIONS.REGION_ID"
    FROM "shop"."regions"
  ),
  "menura_8_join" AS (
    SELECT
      "AMOUNT",
      "REGION_ID",
      "menura.row_identity",
      "SALES_REGIONS.NAME",
      "SALES_REGIONS.REGION_ID"
    FROM "menura_6_scan"
    LEFT JOIN "menura_7_scan" ON "menura_6_scan"."REGION_ID" = "menura_7_scan"."SALES_REGIONS.REGION_ID"
  ),
  "menura_9_deduplicate" AS (
    SELECT
      "AMOUNT",
      "REGION_ID",
      "menura.row_identity",
      "SALES_REGIONS.NAME",
      "SALES_REGIONS.REGION_ID"
    FROM "menura_8_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "SALES_REGIONS.NAME", "menura.row_identity") = 1
  ),
  "menura_10_aggregate" AS (
    SELECT
      "SALES_REGIONS.NAME" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_2_component_1"
    FROM "menura_9_deduplicate"
    GROUP BY 1
  ),
  "menura_11_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_5_aggregate"
  ),
  "menura_12_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_10_aggregate"
  ),
  "menura_13_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_5_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_10_aggregate"
  ),
  "menura_14_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_1_component_1" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "metric_1_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1"
    FROM "menura_13_keys" AS "menura_keys"
    LEFT JOIN "menura_11_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_12_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_15_project" AS (
    SELECT
      "dimension_1" AS "name",
      "metric_1_component_1" AS "budget_count",
      "metric_2_component_1" AS "total_sales"
    FROM "menura_14_stitch"
  )
SELECT
  "name",
  "budget_count",
  "total_sales"
FROM "menura_15_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1