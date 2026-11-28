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
  "menura_3_join" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row_identity",
      "VISIT_TIER.CUSTOMER_ID",
      "VISIT_TIER.TIER",
      "VISIT_TIER.TIER_FROM",
      "VISIT_TIER.TIER_TO"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON ("menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."VISIT_TIER.CUSTOMER_ID") AND ("menura_2_scan"."VISIT_TIER.TIER_FROM" IS NOT NULL) AND ("menura_2_scan"."VISIT_TIER.TIER_FROM" <= "menura_1_scan"."VISITED_AT") AND (("menura_2_scan"."VISIT_TIER.TIER_TO" IS NULL OR "menura_1_scan"."VISITED_AT" <= "menura_2_scan"."VISIT_TIER.TIER_TO"))
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "VISITED_AT",
      "VISIT_ID",
      "menura.row_identity",
      "VISIT_TIER.CUSTOMER_ID",
      "VISIT_TIER.TIER",
      "VISIT_TIER.TIER_FROM",
      "VISIT_TIER.TIER_TO"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "VISIT_ID", "VISIT_TIER.TIER", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "VISIT_ID" AS "dimension_1",
      "VISIT_TIER.TIER" AS "dimension_2",
      CAST(COUNT("VISIT_ID") AS BIGINT) AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1, 2
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "visit_id",
      "dimension_2" AS "tier",
      "metric_1_component_1" AS "visit_count"
    FROM "menura_5_aggregate"
  )
SELECT
  "visit_id",
  "tier",
  "visit_count"
FROM "menura_6_project"