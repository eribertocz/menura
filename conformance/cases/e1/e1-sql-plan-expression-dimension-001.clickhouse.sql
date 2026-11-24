WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "channel" AS "CHANNEL",
      "quantity" AS "QUANTITY"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "CHANNEL",
      "QUANTITY"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "AMOUNT",
      "CHANNEL",
      "QUANTITY"
    FROM "menura_2_filter"
    WHERE "QUANTITY" > {p1:Int64}
  ),
  "menura_4_project" AS (
    SELECT
      "AMOUNT" AS "AMOUNT",
      "CHANNEL" AS "CHANNEL",
      "QUANTITY" AS "QUANTITY",
      CAST(CASE WHEN ("CHANNEL" = {p2:String}) THEN {p3:String} ELSE {p4:String} END AS Nullable(String)) AS "menura.dimension_1"
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
      "dimension_1" AS "kind",
      "metric_1_component_1" AS "amount"
    FROM "menura_5_aggregate"
  ),
  "menura_7_sort" AS (
    SELECT
      "kind",
      "amount"
    FROM "menura_6_project"
    ORDER BY "kind" ASC NULLS LAST
  )
SELECT
  "kind",
  "amount"
FROM "menura_7_sort"
ORDER BY "kind" ASC NULLS LAST
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1