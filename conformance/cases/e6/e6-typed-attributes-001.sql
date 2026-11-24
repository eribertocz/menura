WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "region" AS "REGION",
      "sale_date" AS "SALE_DATE",
      "sale_id" AS "SALE_ID",
      "voided" AS "VOIDED"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "REGION",
      "SALE_DATE",
      "SALE_ID",
      "VOIDED"
    FROM "menura_1_scan"
    WHERE TRUE AND ((("AMOUNT" >= CAST($1 AS DECIMAL(10,2))) AND ("SALE_DATE" >= CAST($2 AS DATE))) AND ("VOIDED" = CAST($3 AS BOOLEAN)))
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1",
      CAST(COUNT("SALE_ID") AS BIGINT) AS "metric_1_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_3_aggregate"
  )
SELECT
  "region",
  "sale_count"
FROM "menura_4_project"