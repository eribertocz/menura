WITH
  "menura_1_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "Amount" AS "menura.source.AMOUNT"
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "FIELD_TOTAL",
      CAST(SUM("menura.source.AMOUNT") AS DECIMAL(38,2)) AS "SOURCE_TOTAL"
    FROM "menura_1_scan"
  ),
  "menura_3_project" AS (
    SELECT
      "FIELD_TOTAL" AS "FIELD_TOTAL",
      "SOURCE_TOTAL" AS "SOURCE_TOTAL"
    FROM "menura_2_aggregate"
  )
SELECT
  "FIELD_TOTAL",
  "SOURCE_TOTAL"
FROM "menura_3_project"
