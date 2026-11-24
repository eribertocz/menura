WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_2"
    FROM "menura_1_scan"
  ),
  "menura_3_scan" AS (
    SELECT
      "amount" AS "AMOUNT"
    FROM "shop"."refunds"
  ),
  "menura_4_aggregate" AS (
    SELECT
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_3_scan"
  ),
  "menura_5_present" AS (
    SELECT
      "metric_1_component_2",
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
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_1_component_2" ELSE NULL END AS "metric_1_component_2",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1"
    FROM "menura_5_present" AS "menura_branch_1"
    LEFT JOIN "menura_6_present" AS "menura_branch_2" ON TRUE
  ),
  "menura_8_project" AS (
    SELECT
      CAST(CASE WHEN toFloat64("metric_1_component_2") = 0 THEN NULL ELSE toFloat64("metric_1_component_1") / toFloat64("metric_1_component_2") END AS Nullable(Float64)) AS "refund_rate"
    FROM "menura_7_stitch"
  )
SELECT
  "refund_rate"
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