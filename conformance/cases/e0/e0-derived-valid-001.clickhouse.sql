WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "region" AS "REGION"
    FROM "shop"."customers"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      accurateCast(coalesce(count(DISTINCT toInt256("CUSTOMER_ID")), 0), 'Nullable(Int64)') AS "metric_3_component_2"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_scan" AS (
    SELECT
      "billed" AS "BILLED",
      "collected" AS "COLLECTED",
      "customer_id" AS "CUSTOMER_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."invoices"
  ),
  "menura_4_scan" AS (
    SELECT
      "customer_id" AS "INVOICES_CUSTOMERS.CUSTOMER_ID",
      "region" AS "INVOICES_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_5_join" AS (
    SELECT
      "BILLED",
      "COLLECTED",
      "CUSTOMER_ID",
      "menura.row_identity",
      "INVOICES_CUSTOMERS.CUSTOMER_ID",
      "INVOICES_CUSTOMERS.REGION"
    FROM "menura_3_scan"
    LEFT JOIN "menura_4_scan" ON "menura_3_scan"."CUSTOMER_ID" = "menura_4_scan"."INVOICES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_6_deduplicate" AS (
    SELECT
      "BILLED",
      "COLLECTED",
      "CUSTOMER_ID",
      "menura.row_identity",
      "INVOICES_CUSTOMERS.CUSTOMER_ID",
      "INVOICES_CUSTOMERS.REGION"
    FROM "menura_5_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "INVOICES_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_7_aggregate" AS (
    SELECT
      "INVOICES_CUSTOMERS.REGION" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("COLLECTED")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("BILLED")), 'Nullable(Int128)') AS "metric_1_component_2",
      accurateCast(sumOrNull(toInt256("COLLECTED")), 'Nullable(Int128)') AS "metric_2_component_1",
      accurateCast(sumOrNull(toInt256("BILLED")), 'Nullable(Int128)') AS "metric_2_component_2",
      accurateCast(sumOrNull(toInt256("BILLED")), 'Nullable(Int128)') AS "metric_3_component_1"
    FROM "menura_6_deduplicate"
    GROUP BY 1
  ),
  "menura_8_present" AS (
    SELECT
      "dimension_1",
      "metric_3_component_2",
      TRUE AS "menura.present"
    FROM "menura_2_aggregate"
  ),
  "menura_9_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "metric_1_component_2",
      "metric_2_component_1",
      "metric_2_component_2",
      "metric_3_component_1",
      TRUE AS "menura.present"
    FROM "menura_7_aggregate"
  ),
  "menura_10_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_2_aggregate"
    UNION DISTINCT
    SELECT
      "dimension_1"
    FROM "menura_7_aggregate"
  ),
  "menura_11_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_3_component_2" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "metric_3_component_2",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_1_component_2" ELSE NULL END AS "metric_1_component_2",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_2_component_2" ELSE NULL END AS "metric_2_component_2",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_3_component_1" ELSE NULL END AS "metric_3_component_1"
    FROM "menura_10_keys" AS "menura_keys"
    LEFT JOIN "menura_8_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_1"."dimension_1")
    LEFT JOIN "menura_9_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."dimension_1", "menura_branch_2"."dimension_1")
  ),
  "menura_12_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_3_component_2" AS "metric_3_component_2",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_1_component_2" AS "metric_1_component_2",
      "metric_2_component_1" AS "metric_2_component_1",
      "metric_2_component_2" AS "metric_2_component_2",
      "metric_3_component_1" AS "metric_3_component_1",
      CAST(CASE WHEN toFloat64("metric_2_component_2") = 0 THEN NULL ELSE toFloat64("metric_2_component_1") / toFloat64("metric_2_component_2") END AS Nullable(Float64)) AS "metric_2_input_1"
    FROM "menura_11_stitch"
  ),
  "menura_13_project" AS (
    SELECT
      "dimension_1" AS "region",
      CAST(CASE WHEN toFloat64("metric_1_component_2") = 0 THEN NULL ELSE toFloat64("metric_1_component_1") / toFloat64("metric_1_component_2") END AS Nullable(Float64)) AS "collection_rate",
      CAST(toInt64(1) - "metric_2_input_1" AS Nullable(Float64)) AS "uncollected_rate",
      CAST(CASE WHEN toFloat64("metric_3_component_2") = 0 THEN throwIf(ifNull(toFloat64("metric_3_component_2") = 0, 0), 'RDER-01: division by zero in metric BILLED_PER_CUSTOMER') ELSE toFloat64("metric_3_component_1") / toFloat64("metric_3_component_2") END AS Nullable(Float64)) AS "billed_per_customer"
    FROM "menura_12_project"
  )
SELECT
  "region",
  "collection_rate",
  "uncollected_rate",
  "billed_per_customer"
FROM "menura_13_project"
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