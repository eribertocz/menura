WITH
  "menura_1_scan" AS (
    SELECT
      "employee_id" AS "EMPLOYEE_ID",
      "store_id" AS "STORE_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."employees"
  ),
  "menura_2_scan" AS (
    SELECT
      "store_id" AS "EMPLOYEES_STORES.STORE_ID",
      "name" AS "EMPLOYEES_STORES.STORE_NAME"
    FROM "shop"."stores"
  ),
  "menura_3_join" AS (
    SELECT
      "EMPLOYEE_ID",
      "STORE_ID",
      "menura.row_identity",
      "EMPLOYEES_STORES.STORE_ID",
      "EMPLOYEES_STORES.STORE_NAME"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."STORE_ID" = "menura_2_scan"."EMPLOYEES_STORES.STORE_ID"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "EMPLOYEE_ID",
      "STORE_ID",
      "menura.row_identity",
      "EMPLOYEES_STORES.STORE_ID",
      "EMPLOYEES_STORES.STORE_NAME"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "EMPLOYEES_STORES.STORE_NAME", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "EMPLOYEES_STORES.STORE_NAME" AS "dimension_1",
      accurateCast(coalesce(count("EMPLOYEE_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "store_name",
      "metric_1_component_1" AS "employee_count"
    FROM "menura_5_aggregate"
  )
SELECT
  "store_name",
  "employee_count"
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