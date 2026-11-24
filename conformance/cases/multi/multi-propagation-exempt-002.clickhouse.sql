WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "store_id" AS "STORE_ID"
    FROM "shop"."customers"
  ),
  "menura_2_scan" AS (
    SELECT
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_3_scan" AS (
    SELECT
      "region" AS "REGION",
      "store_id" AS "STORE_ID"
    FROM "shop"."stores"
  ),
  "menura_4_filter" AS (
    SELECT
      "REGION",
      "STORE_ID"
    FROM "menura_3_scan"
    WHERE (CASE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END) AND TRUE
  ),
  "menura_5_related" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_6_visible" AS (
    SELECT
      "STORE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_4_filter"
    GROUP BY 1
  ),
  "menura_7_semijoin" AS (
    SELECT
      "menura_row"."CUSTOMER_ID",
      "menura_row"."STORE_ID"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."STORE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_6_visible" AS "menura_visible" ON "menura_row"."STORE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_8_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("CUSTOMER_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_7_semijoin"
  ),
  "menura_9_project" AS (
    SELECT
      "metric_1_component_1" AS "customer_count"
    FROM "menura_8_aggregate"
  )
SELECT
  "customer_count"
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