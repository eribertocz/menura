WITH
  "menura_1_scan" AS (
    SELECT
      "channel" AS "CHANNEL",
      "quantity" AS "QUANTITY",
      "sale_id" AS "SALE_ID",
      "voided" AS "VOIDED"
    FROM "shop"."main"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "CHANNEL",
      "QUANTITY",
      "SALE_ID",
      "VOIDED"
    FROM "menura_1_scan"
    WHERE ("CHANNEL" NOT IN (SELECT unnest($1))) AND ("VOIDED" = FALSE)
  ),
  "menura_3_filter" AS (
    SELECT
      "CHANNEL",
      "QUANTITY",
      "SALE_ID",
      "VOIDED"
    FROM "menura_2_filter"
    WHERE "QUANTITY" >= $2
  ),
  "menura_4_aggregate" AS (
    SELECT
      "CHANNEL" AS "dimension_1",
      CAST(COUNT("SALE_ID") AS BIGINT) AS "metric_1_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "channel",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_4_aggregate"
  ),
  "menura_6_sort" AS (
    SELECT
      "channel",
      "sale_count"
    FROM "menura_5_project"
    ORDER BY "channel" DESC NULLS LAST
  )
SELECT
  "channel",
  "sale_count"
FROM "menura_6_sort"
ORDER BY "channel" DESC NULLS LAST