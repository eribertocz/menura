WITH
  "menura_1_scan" AS (
    SELECT
      "region" AS "REGION"
    FROM "shop"."sales"
  ),
  "menura_2_filter" AS (
    SELECT
      "REGION"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "dimension_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "region"
    FROM "menura_3_aggregate"
  )
SELECT
  "region"
FROM "menura_4_project"