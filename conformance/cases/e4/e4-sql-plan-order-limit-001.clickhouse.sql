WITH
  "menura_1_scan" AS (
    SELECT
      "region" AS "REGION",
      "sale_id" AS "SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "REGION",
      "SALE_ID"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "metric_1_component_1" AS "n",
      "dimension_1" AS "region"
    FROM "menura_3_aggregate"
  ),
  "menura_5_sort" AS (
    SELECT
      "n",
      "region"
    FROM "menura_4_project"
    ORDER BY "region" DESC NULLS LAST
  ),
  "menura_6_limit" AS (
    SELECT
      "n",
      "region"
    FROM "menura_5_sort"
    ORDER BY "region" DESC NULLS LAST
    LIMIT 2
  )
SELECT
  "n",
  "region"
FROM "menura_6_limit"
ORDER BY "region" DESC NULLS LAST
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1