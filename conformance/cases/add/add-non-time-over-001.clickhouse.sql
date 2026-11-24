WITH
  "menura_1_scan" AS (
    SELECT
      "product" AS "PRODUCT",
      "quantity" AS "QUANTITY",
      "warehouse" AS "WAREHOUSE"
    FROM "shop"."stock"
  ),
  "menura_2_values" AS (
    SELECT
      "PRODUCT" AS "entity_1",
      "WAREHOUSE" AS "menura.over",
      sumOrNull(toInt256("QUANTITY")) AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1, 2
  ),
  "menura_3_snapshot" AS (
    SELECT
      "entity_1",
      accurateCast(MAX("metric_1_component_1"), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_2_values"
    GROUP BY 1
  ),
  "menura_4_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("metric_1_component_1")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_3_snapshot"
  ),
  "menura_5_project" AS (
    SELECT
      "metric_1_component_1" AS "peak_stock"
    FROM "menura_4_aggregate"
  )
SELECT
  "peak_stock"
FROM "menura_5_project"
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