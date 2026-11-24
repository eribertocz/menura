WITH
  "menura_1_scan" AS (
    SELECT
      "channel" AS "CHANNEL",
      "region" AS "REGION"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "CHANNEL",
      "REGION"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "CHANNEL",
      "REGION"
    FROM "menura_2_filter"
    WHERE "REGION" = {p1:String}
  ),
  "menura_4_aggregate" AS (
    SELECT
      "CHANNEL" AS "dimension_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "channel"
    FROM "menura_4_aggregate"
  )
SELECT
  "channel"
FROM "menura_5_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1