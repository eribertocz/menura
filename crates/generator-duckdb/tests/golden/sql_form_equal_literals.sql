WITH
  "menura_1_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "quantity" AS "QUANTITY"
    FROM "main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "QUANTITY"
    FROM "menura_1_scan"
    WHERE "QUANTITY" > CAST($1 AS BIGINT)
  ),
  "menura_3_project" AS (
    SELECT
      "AMOUNT" AS "AMOUNT",
      "QUANTITY" AS "QUANTITY",
      CAST(CASE WHEN ("QUANTITY" > CAST($2 AS BIGINT)) THEN CAST($3 AS VARCHAR) ELSE CAST($4 AS VARCHAR) END AS VARCHAR) AS "menura.dimension_1"
    FROM "menura_2_filter"
  ),
  "menura_4_aggregate" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "metric_1_component_1"
    FROM "menura_3_project"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "size",
      "metric_1_component_1" AS "amount"
    FROM "menura_4_aggregate"
  )
SELECT
  "size",
  "amount"
FROM "menura_5_project"