WITH
  "menura_1_scan" AS (
    SELECT
      "order_id" AS "ORDER_ID",
      "region" AS "REGION"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      accurateCast(coalesce(count("ORDER_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "orders"
    FROM "menura_2_aggregate"
  ),
  "menura_4_sort" AS (
    SELECT
      "region",
      "orders"
    FROM "menura_3_project"
    ORDER BY "region" ASC NULLS LAST
  )
SELECT
  "region",
  "orders"
FROM "menura_4_sort"
ORDER BY "region" ASC NULLS LAST
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