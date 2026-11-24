WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "status" AS "STATUS"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "STATUS"
    FROM "menura_1_scan"
    WHERE TRUE AND ("STATUS" <> 'void')
  ),
  "menura_3_aggregate" AS (
    SELECT
      "STATUS" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "status" AS "STATUS"
    FROM "shop"."orders"
  ),
  "menura_5_filter" AS (
    SELECT
      "AMOUNT",
      "STATUS"
    FROM "menura_4_scan"
    WHERE TRUE AND ("STATUS" <> 'void')
  ),
  "menura_6_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_5_filter"
  ),
  "menura_7_present" AS (
    SELECT
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_6_aggregate"
  ),
  "menura_8_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_3_aggregate" AS "menura_result"
    LEFT JOIN "menura_7_present" AS "menura_context" ON TRUE
  ),
  "menura_9_project" AS (
    SELECT
      "dimension_1" AS "status",
      "metric_1_component_1" AS "sales",
      CAST(CASE WHEN toFloat64("context_1_component_1") = 0 THEN NULL ELSE toFloat64("metric_2_component_1") / toFloat64("context_1_component_1") END AS Nullable(Float64)) AS "sales_share_of_all"
    FROM "menura_8_lookup"
  )
SELECT
  "status",
  "sales",
  "sales_share_of_all"
FROM "menura_9_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1