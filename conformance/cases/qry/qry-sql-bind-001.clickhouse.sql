WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "channel" AS "CHANNEL",
      "region" AS "REGION"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CHANNEL",
      "REGION"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "AMOUNT",
      "CHANNEL",
      "REGION"
    FROM "menura_2_filter"
    WHERE "REGION" = {p1:String}
  ),
  "menura_4_project" AS (
    SELECT
      "AMOUNT" AS "AMOUNT",
      "CHANNEL" AS "CHANNEL",
      "REGION" AS "REGION",
      CAST(CAST(upperUTF8("CHANNEL") AS Nullable(String)) AS Nullable(String)) AS "menura.dimension_1"
    FROM "menura_3_filter"
  ),
  "menura_5_aggregate" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_4_project"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "c",
      "metric_1_component_1" AS "amount"
    FROM "menura_5_aggregate"
  ),
  "menura_7_sort" AS (
    SELECT
      "c",
      "amount"
    FROM "menura_6_project"
    ORDER BY "c" ASC NULLS LAST
  )
SELECT
  "c",
  "amount"
FROM "menura_7_sort"
ORDER BY "c" ASC NULLS LAST
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