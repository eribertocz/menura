WITH
  "menura_1_scan" AS (
    SELECT
      "region" AS "REGION",
      "sale_date" AS "SALE_DATE",
      "sale_id" AS "SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "REGION",
      "SALE_DATE",
      "SALE_ID"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "REGION",
      "SALE_DATE",
      "SALE_ID"
    FROM "menura_2_filter"
    WHERE ("SALE_DATE" >= {p1:Date32}) AND ("SALE_DATE" <= {p2:Date32})
  ),
  "menura_4_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_4_aggregate"
  )
SELECT
  "region",
  "sale_count"
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