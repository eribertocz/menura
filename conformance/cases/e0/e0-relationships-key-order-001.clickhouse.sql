WITH
  "menura_1_scan" AS (
    SELECT
      "date_key" AS "DATE_KEY",
      "packages" AS "PACKAGES",
      "store_id" AS "STORE_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."deliveries"
  ),
  "menura_2_scan" AS (
    SELECT
      "date_key" AS "DELIVERIES_STORE_DAYS.DATE_KEY",
      "opening_hours" AS "DELIVERIES_STORE_DAYS.OPENING_HOURS",
      "store_id" AS "DELIVERIES_STORE_DAYS.STORE_ID"
    FROM "shop"."store_days"
  ),
  "menura_3_join" AS (
    SELECT
      "DATE_KEY",
      "PACKAGES",
      "STORE_ID",
      "menura.row_identity",
      "DELIVERIES_STORE_DAYS.DATE_KEY",
      "DELIVERIES_STORE_DAYS.OPENING_HOURS",
      "DELIVERIES_STORE_DAYS.STORE_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON ("menura_1_scan"."DATE_KEY" = "menura_2_scan"."DELIVERIES_STORE_DAYS.DATE_KEY") AND ("menura_1_scan"."STORE_ID" = "menura_2_scan"."DELIVERIES_STORE_DAYS.STORE_ID")
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "DATE_KEY",
      "PACKAGES",
      "STORE_ID",
      "menura.row_identity",
      "DELIVERIES_STORE_DAYS.DATE_KEY",
      "DELIVERIES_STORE_DAYS.OPENING_HOURS",
      "DELIVERIES_STORE_DAYS.STORE_ID"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "DELIVERIES_STORE_DAYS.OPENING_HOURS", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "DELIVERIES_STORE_DAYS.OPENING_HOURS" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("PACKAGES")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "opening_hours",
      "metric_1_component_1" AS "total_packages"
    FROM "menura_5_aggregate"
  )
SELECT
  "opening_hours",
  "total_packages"
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