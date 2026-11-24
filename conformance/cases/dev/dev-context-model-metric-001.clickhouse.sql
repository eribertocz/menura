WITH
  "menura_1_scan" AS (
    SELECT
      "billed" AS "BILLED",
      "customer_id" AS "CUSTOMER_ID"
    FROM "shop"."invoices"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "CUSTOMER_ID" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("BILLED")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("BILLED")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_scan" AS (
    SELECT
      "billed" AS "BILLED"
    FROM "shop"."invoices"
  ),
  "menura_4_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("BILLED")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_3_scan"
  ),
  "menura_5_present" AS (
    SELECT
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_4_aggregate"
  ),
  "menura_6_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_2_aggregate" AS "menura_result"
    LEFT JOIN "menura_5_present" AS "menura_context" ON TRUE
  ),
  "menura_7_project" AS (
    SELECT
      "dimension_1" AS "customer_id",
      "metric_1_component_1" AS "billed",
      CAST(CASE WHEN toFloat64("context_1_component_1") = 0 THEN NULL ELSE toFloat64("metric_2_component_1") / toFloat64("context_1_component_1") END AS Nullable(Float64)) AS "billed_share"
    FROM "menura_6_lookup"
  )
SELECT
  "customer_id",
  "billed",
  "billed_share"
FROM "menura_7_project"
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