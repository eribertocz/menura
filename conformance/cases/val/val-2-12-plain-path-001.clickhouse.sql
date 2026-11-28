WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "sale_id" AS "SALE_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."sales"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "SALE_CUSTOMER_ANY.CUSTOMER_ID",
      "segment" AS "SALE_CUSTOMER_ANY.SEGMENT"
    FROM "shop"."customer_versions"
  ),
  "menura_3_join" AS (
    SELECT
      "CUSTOMER_ID",
      "SALE_ID",
      "menura.row_identity",
      "SALE_CUSTOMER_ANY.CUSTOMER_ID",
      "SALE_CUSTOMER_ANY.SEGMENT"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."SALE_CUSTOMER_ANY.CUSTOMER_ID"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "SALE_ID",
      "menura.row_identity",
      "SALE_CUSTOMER_ANY.CUSTOMER_ID",
      "SALE_CUSTOMER_ANY.SEGMENT"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "SALE_CUSTOMER_ANY.SEGMENT", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "SALE_CUSTOMER_ANY.SEGMENT" AS "dimension_1",
      accurateCast(coalesce(count("SALE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "segment",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_5_aggregate"
  )
SELECT
  "segment",
  "sale_count"
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