WITH
  "menura_1_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "QUANTITY",
      "REGION"
    FROM "menura_1_scan"
    WHERE "QUANTITY" > CAST($1 AS BIGINT)
  ),
  "menura_3_project" AS (
    SELECT
      "AMOUNT" AS "AMOUNT",
      "QUANTITY" AS "QUANTITY",
      "REGION" AS "REGION",
      CAST(CASE WHEN ("REGION" = CAST($2 AS VARCHAR)) THEN CAST($3 AS VARCHAR) ELSE "REGION" END AS VARCHAR) AS "menura.dimension_1"
    FROM "menura_2_filter"
  ),
  "menura_4_aggregate" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_2_component_1"
    FROM "menura_3_project"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST("metric_2_component_1" * 100 AS DECIMAL(38,2)) AS "metric_2"
    FROM "menura_4_aggregate"
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "r",
      CAST(CAST(coalesce(CAST("metric_1_component_1" AS DECIMAL(38,2)), CAST(CAST($4 AS BIGINT) AS DECIMAL(38,2))) AS DECIMAL(38,2)) AS DECIMAL(38,2)) AS "padded",
      "metric_2" AS "scaled"
    FROM "menura_5_project"
  )
SELECT
  "r",
  "padded",
  "scaled"
FROM "menura_6_project"