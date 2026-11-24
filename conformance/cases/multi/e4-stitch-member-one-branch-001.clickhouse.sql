WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
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
      "AMOUNT",
      "REGION_ID",
      "menura.row_identity",
      "BUDGETS_REGIONS.NAME",
      "BUDGETS_REGIONS.REGION_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."REGION_ID" = "menura_2_scan"."BUDGETS_REGIONS.REGION_ID"
  ),
  "menura_4_filter" AS (
    SELECT
      "AMOUNT",
      "REGION_ID",
      "menura.row_identity",
      "BUDGETS_REGIONS.NAME",
      "BUDGETS_REGIONS.REGION_ID"
    FROM "menura_3_join"
    WHERE "BUDGETS_REGIONS.NAME" <> {p1:String}
  ),
  "menura_5_deduplicate" AS (
    SELECT
      "AMOUNT",
      "REGION_ID",
      "menura.row_identity",
      "BUDGETS_REGIONS.NAME",
      "BUDGETS_REGIONS.REGION_ID"
    FROM "menura_4_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BUDGETS_REGIONS.NAME", "menura.row_identity") = 1
  ),
  "menura_6_aggregate" AS (
    SELECT
      "BUDGETS_REGIONS.NAME" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_2_component_1"
    FROM "menura_5_deduplicate"
    GROUP BY 1
  ),
  "menura_7_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "region_id" AS "REGION_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."sales"
  ),
  "menura_8_scan" AS (
    SELECT
      "name" AS "SALES_REGIONS.NAME",
      "region_id" AS "SALES_REGIONS.REGION_ID"
    FROM "shop"."regions"
  ),
  "menura_9_join" AS (
    SELECT
      "AMOUNT",
      "REGION_ID",
      "menura.row_identity",
      "SALES_REGIONS.NAME",
      "SALES_REGIONS.REGION_ID"
    FROM "menura_7_scan"
    LEFT JOIN "menura_8_scan" ON "menura_7_scan"."REGION_ID" = "menura_8_scan"."SALES_REGIONS.REGION_ID"
  ),
  "menura_10_filter" AS (
    SELECT
      "AMOUNT",
      "REGION_ID",
      "menura.row_identity",
      "SALES_REGIONS.NAME",
      "SALES_REGIONS.REGION_ID"
    FROM "menura_9_join"
    WHERE "SALES_REGIONS.NAME" <> {p1:String}
  ),
  "menura_11_deduplicate" AS (
    SELECT
      "AMOUNT",
      "REGION_ID",
      "menura.row_identity",
      "SALES_REGIONS.NAME",
      "SALES_REGIONS.REGION_ID"
    FROM "menura_10_filter"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "SALES_REGIONS.NAME", "menura.row_identity") = 1
  ),
  "menura_12_aggregate" AS (
    SELECT
      "SALES_REGIONS.NAME" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_11_deduplicate"
    GROUP BY 1
  ),
  "menura_13_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_6_aggregate"
  ),
  "menura_14_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_12_aggregate"
  ),
  "menura_15_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_6_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_12_aggregate"
  ),
  "menura_16_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1"
    FROM "menura_15_keys" AS "menura_keys"
    LEFT JOIN "menura_13_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_14_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_17_project" AS (
    SELECT
      "dimension_1" AS "name",
      "metric_1_component_1" AS "total_sales",
      "metric_2_component_1" AS "budget_total"
    FROM "menura_16_stitch"
  )
SELECT
  "name",
  "total_sales",
  "budget_total"
FROM "menura_17_project"
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