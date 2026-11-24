WITH
  "menura_1_scan" AS (
    SELECT
      "sale_date" AS "SALE_DATE",
      "sale_id" AS "SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "SALE_DATE",
      "SALE_ID"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      toDate32(toMonday("SALE_DATE")) AS "dimension_1",
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "sale_date",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_3_aggregate"
  )
SELECT
  "sale_date",
  "sale_count"
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