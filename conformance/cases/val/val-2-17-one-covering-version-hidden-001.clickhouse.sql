WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "visited_at" AS "VISITED_AT",
      "visit_id" AS "VISIT_ID"
    FROM "shop"."visits"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "tier_from" AS "TIER_FROM",
      "tier_to" AS "TIER_TO"
    FROM "shop"."customer_tiers"
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "tier" AS "TIER",
      "tier_from" AS "TIER_FROM",
      "tier_to" AS "TIER_TO"
    FROM "shop"."customer_tiers"
  ),
  "menura_4_filter" AS (
    SELECT
      "CUSTOMER_ID",
      "TIER",
      "TIER_FROM",
      "TIER_TO"
    FROM "menura_3_scan"
    WHERE ("TIER" = 'SILVER') AND TRUE
  ),
  "menura_5_rows" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      ROW_NUMBER() OVER () AS "menura.row"
    FROM "menura_1_scan"
  ),
  "menura_6_versions" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_row"."VISITED_AT" AS "VISITED_AT",
      "menura_row"."VISIT_ID" AS "VISIT_ID",
      "menura_row"."menura.row" AS "menura.row",
      CASE WHEN "menura_version"."TIER_FROM" IS NOT NULL AND "menura_version"."TIER_FROM" <= "menura_row"."VISITED_AT" AND ("menura_version"."TIER_TO" IS NULL OR "menura_row"."VISITED_AT" <= "menura_version"."TIER_TO") THEN 1 ELSE 0 END AS "menura.covers"
    FROM "menura_5_rows" AS "menura_row"
    LEFT JOIN "menura_2_scan" AS "menura_version" ON "menura_row"."CUSTOMER_ID" = "menura_version"."CUSTOMER_ID"
  ),
  "menura_7_covered" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row",
      SUM("menura.covers") OVER (PARTITION BY "menura.row") AS "menura.related",
      ROW_NUMBER() OVER (PARTITION BY "menura.row") AS "menura.rank"
    FROM "menura_6_versions"
  ),
  "menura_8_counted" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row",
      "menura.related"
    FROM "menura_7_covered"
    WHERE "menura.rank" = 1
  ),
  "menura_9_versions" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_row"."VISITED_AT" AS "VISITED_AT",
      "menura_row"."VISIT_ID" AS "VISIT_ID",
      "menura_row"."menura.row" AS "menura.row",
      "menura_row"."menura.related" AS "menura.related",
      CASE WHEN "menura_version"."TIER_FROM" IS NOT NULL AND "menura_version"."TIER_FROM" <= "menura_row"."VISITED_AT" AND ("menura_version"."TIER_TO" IS NULL OR "menura_row"."VISITED_AT" <= "menura_version"."TIER_TO") THEN 1 ELSE 0 END AS "menura.covers"
    FROM "menura_8_counted" AS "menura_row"
    LEFT JOIN "menura_4_filter" AS "menura_version" ON "menura_row"."CUSTOMER_ID" = "menura_version"."CUSTOMER_ID"
  ),
  "menura_10_covered" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row",
      "menura.related",
      SUM("menura.covers") OVER (PARTITION BY "menura.row") AS "menura.visible",
      ROW_NUMBER() OVER (PARTITION BY "menura.row") AS "menura.rank"
    FROM "menura_9_versions"
  ),
  "menura_11_counted" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row",
      "menura.related",
      "menura.visible"
    FROM "menura_10_covered"
    WHERE "menura.rank" = 1
  ),
  "menura_12_semijoin" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID"
    FROM "menura_11_counted"
    WHERE "menura.related" >= 1 AND "menura.related" = "menura.visible"
  ),
  "menura_13_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("VISIT_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_12_semijoin"
  ),
  "menura_14_project" AS (
    SELECT
      "metric_1_component_1" AS "visit_count"
    FROM "menura_13_aggregate"
  )
SELECT
  "visit_count"
FROM "menura_14_project"
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