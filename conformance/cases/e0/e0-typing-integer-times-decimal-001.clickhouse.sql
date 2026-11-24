WITH
  "menura_1_scan" AS (
    SELECT
      "price" AS "PRICE",
      CASE WHEN ("price" < toInt64(8)) THEN 'low' ELSE 'high' END AS "PRICE_BAND",
      "quantity" AS "QUANTITY"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "PRICE_BAND" AS "dimension_1",
      CAST(sumOrNull("PRICE" * "QUANTITY") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "price_band",
      "metric_1_component_1" AS "total_line_price"
    FROM "menura_2_aggregate"
  )
SELECT
  "price_band",
  "total_line_price"
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