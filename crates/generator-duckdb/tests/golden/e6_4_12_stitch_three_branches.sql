WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(COUNT(*) AS BIGINT) AS "ORDER_COUNT",
      CAST(SUM("QUANTITY") AS HUGEINT) AS "TOTAL_QUANTITY"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_4_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "TOTAL_AMOUNT"
    FROM "menura_3_scan"
    GROUP BY 1
  ),
  "menura_5_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_6_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(COUNT(DISTINCT "QUANTITY") AS BIGINT) AS "DISTINCT_QUANTITIES"
    FROM "menura_5_scan"
    GROUP BY 1
  ),
  "menura_7_present" AS (
    SELECT
      "REGION",
      "ORDER_COUNT",
      "TOTAL_QUANTITY",
      TRUE AS "menura.present"
    FROM "menura_2_aggregate"
  ),
  "menura_8_present" AS (
    SELECT
      "REGION",
      "TOTAL_AMOUNT",
      TRUE AS "menura.present"
    FROM "menura_4_aggregate"
  ),
  "menura_9_present" AS (
    SELECT
      "REGION",
      "DISTINCT_QUANTITIES",
      TRUE AS "menura.present"
    FROM "menura_6_aggregate"
  ),
  "menura_10_keys" AS (
    SELECT
      "REGION"
    FROM "menura_2_aggregate"
    UNION
    SELECT
      "REGION"
    FROM "menura_4_aggregate"
    UNION
    SELECT
      "REGION"
    FROM "menura_6_aggregate"
  ),
  "menura_11_stitch" AS (
    SELECT
      "menura_keys"."REGION" AS "REGION",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."ORDER_COUNT" ELSE CAST(0 AS BIGINT) END AS "ORDER_COUNT",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."TOTAL_QUANTITY" ELSE NULL END AS "TOTAL_QUANTITY",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."TOTAL_AMOUNT" ELSE NULL END AS "TOTAL_AMOUNT",
      CASE WHEN "menura_branch_3"."menura.present" THEN "menura_branch_3"."DISTINCT_QUANTITIES" ELSE CAST(0 AS BIGINT) END AS "DISTINCT_QUANTITIES"
    FROM "menura_10_keys" AS "menura_keys"
    LEFT JOIN "menura_7_present" AS "menura_branch_1" ON "menura_keys"."REGION" IS NOT DISTINCT FROM "menura_branch_1"."REGION"
    LEFT JOIN "menura_8_present" AS "menura_branch_2" ON "menura_keys"."REGION" IS NOT DISTINCT FROM "menura_branch_2"."REGION"
    LEFT JOIN "menura_9_present" AS "menura_branch_3" ON "menura_keys"."REGION" IS NOT DISTINCT FROM "menura_branch_3"."REGION"
  )
SELECT
  "REGION",
  "ORDER_COUNT",
  "TOTAL_QUANTITY",
  "TOTAL_AMOUNT",
  "DISTINCT_QUANTITIES"
FROM "menura_11_stitch"