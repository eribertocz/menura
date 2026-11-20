WITH
  "menura_1_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "created_at" AS "CREATED_AT",
      "ordered_on" AS "ORDERED_ON",
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_project" AS (
    SELECT
      CAST(CAST(date_part('week', ("CREATED_AT" AT TIME ZONE $1)) AS BIGINT) AS BIGINT) AS "WEEK",
      CAST(CAST("ORDERED_ON" + to_months(CAST($2 AS INTEGER)) AS DATE) AS DATE) AS "LATER",
      CAST(CAST((CAST(date_trunc('week', $3) AS DATE) - CAST(date_trunc('week', "ORDERED_ON") AS DATE)) // 7 AS BIGINT) AS BIGINT) AS "WEEKS",
      CAST(CAST($4 AT TIME ZONE $1 AS TIMESTAMP) AS TIMESTAMP) AS "NOW"
    FROM "menura_1_scan"
  )
SELECT
  "WEEK",
  "LATER",
  "WEEKS",
  "NOW"
FROM "menura_2_project"