WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "QUANTITY",
      "REGION"
    FROM "menura_1_scan"
    WHERE "REGION" = CAST($1 AS VARCHAR)
  ),
  "menura_3_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT",
      CAST(SUM("QUANTITY") AS HUGEINT) AS "TOTAL_QUANTITY"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_5_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "TOTAL_AMOUNT"
    FROM "menura_4_scan"
    GROUP BY 1
  ),
  "menura_6_present" AS (
    SELECT
      "REGION",
      "ORDER_COUNT",
      "TOTAL_QUANTITY",
      TRUE AS "menura.present"
    FROM "menura_3_aggregate"
  ),
  "menura_7_present" AS (
    SELECT
      "REGION",
      "TOTAL_AMOUNT",
      TRUE AS "menura.present"
    FROM "menura_5_aggregate"
  ),
  "menura_8_keys" AS (
    SELECT
      "REGION"
    FROM "menura_3_aggregate"
    UNION
    SELECT
      "REGION"
    FROM "menura_5_aggregate"
  ),
  "menura_9_stitch" AS (
    SELECT
      "menura_keys"."REGION" AS "REGION",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."ORDER_COUNT" ELSE CAST(0 AS BIGINT) END AS "ORDER_COUNT",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."TOTAL_QUANTITY" ELSE NULL END AS "TOTAL_QUANTITY",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."TOTAL_AMOUNT" ELSE NULL END AS "TOTAL_AMOUNT"
    FROM "menura_8_keys" AS "menura_keys"
    LEFT JOIN "menura_6_present" AS "menura_branch_1" ON "menura_keys"."REGION" IS NOT DISTINCT FROM "menura_branch_1"."REGION"
    LEFT JOIN "menura_7_present" AS "menura_branch_2" ON "menura_keys"."REGION" IS NOT DISTINCT FROM "menura_branch_2"."REGION"
  )
SELECT
  "REGION",
  "ORDER_COUNT",
  "TOTAL_QUANTITY",
  "TOTAL_AMOUNT"
FROM "menura_9_stitch"