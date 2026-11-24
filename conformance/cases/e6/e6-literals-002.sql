WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "sale_date" AS "SALE_DATE",
      "sale_id" AS "SALE_ID",
      "voided" AS "VOIDED"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "QUANTITY",
      "SALE_DATE",
      "SALE_ID",
      "VOIDED"
    FROM "menura_1_scan"
    WHERE FALSE AND ("VOIDED" = FALSE)
  ),
  "menura_3_filter" AS (
    SELECT
      "QUANTITY",
      "SALE_DATE",
      "SALE_ID",
      "VOIDED"
    FROM "menura_2_filter"
    WHERE "QUANTITY" >= CAST($1 AS BIGINT)
  ),
  "menura_4_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "SALE_DATE") AS DATE) AS "dimension_1",
      CAST(COUNT("SALE_ID") AS BIGINT) AS "metric_1_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "sale_date",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_4_aggregate"
  ),
  "menura_6_sort" AS (
    SELECT
      "sale_date",
      "sale_count"
    FROM "menura_5_project"
    ORDER BY "sale_date" ASC NULLS LAST
  ),
  "menura_7_limit" AS (
    SELECT
      "sale_date",
      "sale_count"
    FROM "menura_6_sort"
    ORDER BY "sale_date" ASC NULLS LAST
    LIMIT 2
  )
SELECT
  "sale_date",
  "sale_count"
FROM "menura_7_limit"
ORDER BY "sale_date" ASC NULLS LAST