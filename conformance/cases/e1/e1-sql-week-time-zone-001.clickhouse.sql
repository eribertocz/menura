WITH
  "menura_1_scan" AS (
    SELECT
      "sale_id" AS "SALE_ID",
      "sold_at" AS "SOLD_AT"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "SALE_ID",
      "SOLD_AT"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_project" AS (
    SELECT
      "SALE_ID" AS "SALE_ID",
      "SOLD_AT" AS "SOLD_AT",
      CAST(toTimeZone(toDateTime64(toMonday("SOLD_AT", {p1:String}), 6, {p1:String}), 'UTC') AS Nullable(DateTime64(6, 'UTC'))) AS "menura.dimension_1"
    FROM "menura_2_filter"
  ),
  "menura_4_aggregate" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_3_project"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "sold_week",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_4_aggregate"
  ),
  "menura_6_sort" AS (
    SELECT
      "sold_week",
      "sale_count"
    FROM "menura_5_project"
    ORDER BY "sold_week" ASC NULLS LAST
  )
SELECT
  "sold_week",
  "sale_count"
FROM "menura_6_sort"
ORDER BY "sold_week" ASC NULLS LAST
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