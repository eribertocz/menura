WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "sale_date" AS "SALE_DATE",
      "sale_id" AS "SALE_ID",
      "voided" AS "VOIDED"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "QUANTITY",
      "SALE_DATE",
      "SALE_ID",
      "VOIDED"
    FROM "menura_1_scan"
    WHERE FALSE AND ("VOIDED" = FALSE)
  ),
  "menura_3_filter" AS (
    SELECT
      "QUANTITY",
      "SALE_DATE",
      "SALE_ID",
      "VOIDED"
    FROM "menura_2_filter"
    WHERE "QUANTITY" >= {p1:Int64}
  ),
  "menura_4_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("SALE_DATE")) AS "dimension_1",
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "sale_date",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_4_aggregate"
  ),
  "menura_6_sort" AS (
    SELECT
      "sale_date",
      "sale_count"
    FROM "menura_5_project"
    ORDER BY "sale_date" ASC NULLS LAST
  ),
  "menura_7_limit" AS (
    SELECT
      "sale_date",
      "sale_count"
    FROM "menura_6_sort"
    ORDER BY "sale_date" ASC NULLS LAST
    LIMIT 2
  )
SELECT
  "sale_date",
  "sale_count"
FROM "menura_7_limit"
ORDER BY "sale_date" ASC NULLS LAST
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