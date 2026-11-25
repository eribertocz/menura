WITH
  "menura_1_scan" AS (
    SELECT
      "item_id" AS "ITEM_ID",
      "shelf_no" AS "SHELF_NO",
      "site_code" AS "SITE_CODE"
    FROM "shop"."items"
  ),
  "menura_2_scan" AS (
    SELECT
      "shelf_no" AS "SHELF_NO",
      "site_code" AS "SITE_CODE"
    FROM "shop"."shelves"
  ),
  "menura_3_scan" AS (
    SELECT
      "region" AS "REGION",
      "shelf_no" AS "SHELF_NO",
      "site_code" AS "SITE_CODE"
    FROM "shop"."shelves"
  ),
  "menura_4_filter" AS (
    SELECT
      "REGION",
      "SHELF_NO",
      "SITE_CODE"
    FROM "menura_3_scan"
    WHERE TRUE AND (CASE WHEN empty({p1:Array(String)}) THEN FALSE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END)
  ),
  "menura_5_related" AS (
    SELECT
      "SITE_CODE" AS "menura.key_1",
      "SHELF_NO" AS "menura.key_2",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1, 2
  ),
  "menura_6_visible" AS (
    SELECT
      "SITE_CODE" AS "menura.key_1",
      "SHELF_NO" AS "menura.key_2",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_4_filter"
    GROUP BY 1, 2
  ),
  "menura_7_semijoin" AS (
    SELECT
      "menura_row"."ITEM_ID" AS "ITEM_ID",
      "menura_row"."SHELF_NO" AS "SHELF_NO",
      "menura_row"."SITE_CODE" AS "SITE_CODE"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."SITE_CODE" = "menura_related"."menura.key_1" AND "menura_row"."SHELF_NO" = "menura_related"."menura.key_2"
    LEFT JOIN "menura_6_visible" AS "menura_visible" ON "menura_row"."SITE_CODE" = "menura_visible"."menura.key_1" AND "menura_row"."SHELF_NO" = "menura_visible"."menura.key_2"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_8_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("ITEM_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_7_semijoin"
  ),
  "menura_9_project" AS (
    SELECT
      "metric_1_component_1" AS "item_count"
    FROM "menura_8_aggregate"
  )
SELECT
  "item_count"
FROM "menura_9_project"
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