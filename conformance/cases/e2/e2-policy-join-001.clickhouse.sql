WITH
  "menura_1_scan" AS (
    SELECT
      "refund" AS "REFUND",
      "sale_id" AS "SALE_ID"
    FROM "shop"."returns"
  ),
  "menura_2_scan" AS (
    SELECT
      "sale_id" AS "SALE_ID"
    FROM "shop"."sales"
  ),
  "menura_3_scan" AS (
    SELECT
      "region" AS "REGION",
      "sale_id" AS "SALE_ID",
      "voided" AS "VOIDED"
    FROM "shop"."sales"
  ),
  "menura_4_filter" AS (
    SELECT
      "REGION",
      "SALE_ID",
      "VOIDED"
    FROM "menura_3_scan"
    WHERE (CASE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END) AND ("VOIDED" = FALSE)
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
      "menura_row"."SALE_ID"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."SALE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_6_visible" AS "menura_visible" ON "menura_row"."SALE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_8_aggregate" AS (
    SELECT
      CAST(sumOrNull("REFUND") AS Nullable(Decimal(38, 2))) AS "metric_1_component_1"
    FROM "menura_7_semijoin"
  ),
  "menura_9_project" AS (
    SELECT
      "metric_1_component_1" AS "total_refund"
    FROM "menura_8_aggregate"
  )
SELECT
  "total_refund"
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