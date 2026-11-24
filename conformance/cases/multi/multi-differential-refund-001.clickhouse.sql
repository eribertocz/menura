WITH
  "menura_1_scan" AS (
    SELECT
      "refund" AS "REFUND",
      "sale_id" AS "SALE_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."returns"
  ),
  "menura_2_scan" AS (
    SELECT
      "sale_id" AS "SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_3_scan" AS (
    SELECT
      "sale_id" AS "SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_4_filter" AS (
    SELECT
      "SALE_ID"
    FROM "menura_3_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_5_related" AS (
    SELECT
      "SALE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_6_visible" AS (
    SELECT
      "SALE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_4_filter"
    GROUP BY 1
  ),
  "menura_7_semijoin" AS (
    SELECT
      "menura_row"."REFUND",
      "menura_row"."SALE_ID",
      "menura_row"."menura.row_identity"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."SALE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_6_visible" AS "menura_visible" ON "menura_row"."SALE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_8_scan" AS (
    SELECT
      "region" AS "RETURNS_SALES.REGION",
      "sale_id" AS "RETURNS_SALES.SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_9_filter" AS (
    SELECT
      "RETURNS_SALES.REGION",
      "RETURNS_SALES.SALE_ID"
    FROM "menura_8_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_10_join" AS (
    SELECT
      "REFUND",
      "SALE_ID",
      "menura.row_identity",
      "RETURNS_SALES.REGION",
      "RETURNS_SALES.SALE_ID"
    FROM "menura_7_semijoin"
    LEFT JOIN "menura_9_filter" ON "menura_7_semijoin"."SALE_ID" = "menura_9_filter"."RETURNS_SALES.SALE_ID"
  ),
  "menura_11_deduplicate" AS (
    SELECT
      "REFUND",
      "SALE_ID",
      "menura.row_identity",
      "RETURNS_SALES.REGION",
      "RETURNS_SALES.SALE_ID"
    FROM "menura_10_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "RETURNS_SALES.REGION", "menura.row_identity") = 1
  ),
  "menura_12_aggregate" AS (
    SELECT
      "RETURNS_SALES.REGION" AS "dimension_1",
      CAST(sumOrNull("REFUND") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_11_deduplicate"
    GROUP BY 1
  ),
  "menura_13_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "total_refund"
    FROM "menura_12_aggregate"
  )
SELECT
  "region",
  "total_refund"
FROM "menura_13_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1