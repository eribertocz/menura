WITH
  "menura_1_scan" AS (
    SELECT
      "budget_id" AS "BUDGET_ID"
    FROM "shop"."budgets"
  ),
  "menura_2_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("BUDGET_ID"), 0), 'Nullable(Int64)') AS "metric_2_component_1"
    FROM "menura_1_scan"
  ),
  "menura_3_scan" AS (
    SELECT
      "amount" AS "AMOUNT"
    FROM "shop"."sales"
  ),
  "menura_4_aggregate" AS (
    SELECT
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_3_scan"
  ),
  "menura_5_present" AS (
    SELECT
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_2_aggregate"
  ),
  "menura_6_present" AS (
    SELECT
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_4_aggregate"
  ),
  "menura_7_stitch" AS (
    SELECT
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_2_component_1" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1"
    FROM "menura_5_present" AS "menura_branch_1"
    LEFT JOIN "menura_6_present" AS "menura_branch_2" ON TRUE
  ),
  "menura_8_project" AS (
    SELECT
      "metric_1_component_1" AS "total_sales",
      "metric_2_component_1" AS "budget_count"
    FROM "menura_7_stitch"
  )
SELECT
  "total_sales",
  "budget_count"
FROM "menura_8_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1