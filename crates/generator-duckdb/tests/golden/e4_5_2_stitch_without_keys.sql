WITH
  "menura_1_scan" AS (
    SELECT
      TRUE AS "menura.source.."
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT"
    FROM "menura_1_scan"
  ),
  "menura_3_scan" AS (
    SELECT
      "Amount" AS "AMOUNT"
    FROM "main"."orders"
  ),
  "menura_4_aggregate" AS (
    SELECT
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "TOTAL_AMOUNT"
    FROM "menura_3_scan"
  ),
  "menura_5_stitch" AS (
    SELECT
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."ORDER_COUNT" ELSE CAST(0 AS BIGINT) END AS "ORDER_COUNT",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."TOTAL_AMOUNT" ELSE NULL END AS "TOTAL_AMOUNT"
    FROM (SELECT TRUE) AS "menura_keys"
    LEFT JOIN (SELECT *, TRUE AS "menura.present" FROM "menura_2_aggregate") AS "menura_branch_1" ON TRUE
    LEFT JOIN (SELECT *, TRUE AS "menura.present" FROM "menura_4_aggregate") AS "menura_branch_2" ON TRUE
  )
SELECT
  "ORDER_COUNT",
  "TOTAL_AMOUNT"
FROM "menura_5_stitch"