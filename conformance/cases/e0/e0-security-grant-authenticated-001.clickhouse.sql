WITH
  "menura_1_scan" AS (
    SELECT
      "note_id" AS "NOTE_ID"
    FROM "shop"."notes"
  ),
  "menura_2_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("NOTE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_1_scan"
  ),
  "menura_3_project" AS (
    SELECT
      "metric_1_component_1" AS "note_count"
    FROM "menura_2_aggregate"
  )
SELECT
  "note_count"
FROM "menura_3_project"
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