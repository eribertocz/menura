WITH
  "menura_1_scan" AS (
    SELECT
      "sale_id" AS "SALE_ID",
      CAST(round("amount", 0) AS Nullable(Decimal(10, 0))) AS "WHOLE_AMOUNT"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "SALE_ID",
      "WHOLE_AMOUNT"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1",
      CAST(sumOrNull("WHOLE_AMOUNT") AS Nullable(Decimal(38, 0))) AS "metric_2_component_1"
    FROM "menura_2_filter"
  ),
  "menura_4_project" AS (
    SELECT
      "metric_1_component_1" AS "sale_count",
      "metric_2_component_1" AS "total_whole_amount"
    FROM "menura_3_aggregate"
  )
SELECT
  "sale_count",
  "total_whole_amount"
FROM "menura_4_project"
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