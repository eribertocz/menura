WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "visited_at" AS "VISITED_AT",
      "visit_id" AS "VISIT_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."visits"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "VISIT_TIER.CUSTOMER_ID",
      "tier" AS "VISIT_TIER.TIER",
      "tier_from" AS "VISIT_TIER.TIER_FROM",
      "tier_to" AS "VISIT_TIER.TIER_TO"
    FROM "shop"."customer_tiers"
  ),
  "menura_3_rows" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row_identity",
      ROW_NUMBER() OVER () AS "menura.row"
    FROM "menura_1_scan"
  ),
  "menura_4_versions" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_row"."VISITED_AT" AS "VISITED_AT",
      "menura_row"."VISIT_ID" AS "VISIT_ID",
      "menura_row"."menura.row_identity" AS "menura.row_identity",
      "menura_row"."menura.row" AS "menura.row",
      "menura_version"."VISIT_TIER.CUSTOMER_ID" AS "VISIT_TIER.CUSTOMER_ID",
      "menura_version"."VISIT_TIER.TIER" AS "VISIT_TIER.TIER",
      "menura_version"."VISIT_TIER.TIER_FROM" AS "VISIT_TIER.TIER_FROM",
      "menura_version"."VISIT_TIER.TIER_TO" AS "VISIT_TIER.TIER_TO",
      CASE WHEN "menura_version"."VISIT_TIER.TIER_FROM" IS NOT NULL AND "menura_version"."VISIT_TIER.TIER_FROM" <= "menura_row"."VISITED_AT" AND ("menura_version"."VISIT_TIER.TIER_TO" IS NULL OR "menura_row"."VISITED_AT" <= "menura_version"."VISIT_TIER.TIER_TO") THEN 1 ELSE 0 END AS "menura.covers"
    FROM "menura_3_rows" AS "menura_row"
    LEFT JOIN "menura_2_scan" AS "menura_version" ON "menura_row"."CUSTOMER_ID" = "menura_version"."VISIT_TIER.CUSTOMER_ID"
  ),
  "menura_5_covered" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row_identity",
      "VISIT_TIER.CUSTOMER_ID",
      "VISIT_TIER.TIER",
      "VISIT_TIER.TIER_FROM",
      "VISIT_TIER.TIER_TO",
      "menura.covers",
      MAX("menura.covers") OVER (PARTITION BY "menura.row") AS "menura.covered",
      ROW_NUMBER() OVER (PARTITION BY "menura.row") AS "menura.rank"
    FROM "menura_4_versions"
  ),
  "menura_6_join" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row_identity",
      CASE WHEN "menura.covers" = 1 THEN "VISIT_TIER.CUSTOMER_ID" ELSE NULL END AS "VISIT_TIER.CUSTOMER_ID",
      CASE WHEN "menura.covers" = 1 THEN "VISIT_TIER.TIER" ELSE NULL END AS "VISIT_TIER.TIER",
      CASE WHEN "menura.covers" = 1 THEN "VISIT_TIER.TIER_FROM" ELSE NULL END AS "VISIT_TIER.TIER_FROM",
      CASE WHEN "menura.covers" = 1 THEN "VISIT_TIER.TIER_TO" ELSE NULL END AS "VISIT_TIER.TIER_TO"
    FROM "menura_5_covered"
    WHERE "menura.covers" = 1 OR ("menura.covered" = 0 AND "menura.rank" = 1)
  ),
  "menura_7_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row_identity",
      "VISIT_TIER.CUSTOMER_ID",
      "VISIT_TIER.TIER",
      "VISIT_TIER.TIER_FROM",
      "VISIT_TIER.TIER_TO"
    FROM "menura_6_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "VISIT_ID", "VISIT_TIER.TIER", "menura.row_identity") = 1
  ),
  "menura_8_aggregate" AS (
    SELECT
      "VISIT_ID" AS "dimension_1",
      "VISIT_TIER.TIER" AS "dimension_2",
      accurateCast(coalesce(count("VISIT_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_7_deduplicate"
    GROUP BY 1, 2
  ),
  "menura_9_project" AS (
    SELECT
      "dimension_1" AS "visit_id",
      "dimension_2" AS "tier",
      "metric_1_component_1" AS "visit_count"
    FROM "menura_8_aggregate"
  )
SELECT
  "visit_id",
  "tier",
  "visit_count"
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