WITH
  "menura_1_scan" AS (
    SELECT
      "channel" AS "CHANNEL",
      "sale_id" AS "SALE_ID",
      "voided" AS "VOIDED"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "CHANNEL",
      "SALE_ID",
      "VOIDED"
    FROM "menura_1_scan"
    WHERE (NOT (CASE WHEN "CHANNEL" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("CHANNEL")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END)) AND ("VOIDED" = FALSE)
  ),
  "menura_3_aggregate" AS (
    SELECT
      "CHANNEL" AS "dimension_1",
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "channel",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_3_aggregate"
  ),
  "menura_5_sort" AS (
    SELECT
      "channel",
      "sale_count"
    FROM "menura_4_project"
    ORDER BY "channel" ASC NULLS FIRST
  )
SELECT
  "channel",
  "sale_count"
FROM "menura_5_sort"
ORDER BY "channel" ASC NULLS FIRST
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1