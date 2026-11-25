WITH
  "menura_1_scan" AS (
    SELECT
      "billed" AS "BILLED",
      "collected" AS "COLLECTED",
      "region_id" AS "REGION_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."invoices"
  ),
  "menura_2_scan" AS (
    SELECT
      "name" AS "INVOICES_REGIONS.NAME",
      "region_id" AS "INVOICES_REGIONS.REGION_ID"
    FROM "shop"."regions"
  ),
  "menura_3_join" AS (
    SELECT
      "BILLED",
      "COLLECTED",
      "REGION_ID",
      "menura.row_identity",
      "INVOICES_REGIONS.NAME",
      "INVOICES_REGIONS.REGION_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."REGION_ID" = "menura_2_scan"."INVOICES_REGIONS.REGION_ID"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "BILLED",
      "COLLECTED",
      "REGION_ID",
      "menura.row_identity",
      "INVOICES_REGIONS.NAME",
      "INVOICES_REGIONS.REGION_ID"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "INVOICES_REGIONS.NAME", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "INVOICES_REGIONS.NAME" AS "dimension_1",
      CAST(sumOrNull("COLLECTED") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1",
      CAST(sumOrNull("BILLED") AS Nullable(Decimal(38, 2))) AS "metric_1_component_2"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "name",
      CAST(CASE WHEN toFloat64("metric_1_component_2") = 0 THEN NULL ELSE toFloat64("metric_1_component_1") / toFloat64("metric_1_component_2") END AS Nullable(Float64)) AS "collection_rate"
    FROM "menura_5_aggregate"
  )
SELECT
  "name",
  "collection_rate"
FROM "menura_6_project"
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