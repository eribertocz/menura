WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION",
      date_trunc('month', "created_at" AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR) AS "CREATED_MONTH"
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      "CREATED_MONTH" AS "CREATED_MONTH",
      CAST(SUM("QUANTITY") AS HUGEINT) AS "TOTAL_QUANTITY"
    FROM "menura_1_scan"
    GROUP BY 1, 2
  ),
  "menura_3_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "Sales Region" AS "REGION",
      date_trunc('month', "created_at" AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR) AS "CREATED_MONTH"
    FROM "main"."orders"
  ),
  "menura_4_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      "CREATED_MONTH" AS "CREATED_MONTH",
      CAST(SUM("AMOUNT") AS DECIMAL(38,2)) AS "TOTAL_AMOUNT"
    FROM "menura_3_scan"
    GROUP BY 1, 2
  ),
  "menura_5_present" AS (
    SELECT
      "REGION",
      "CREATED_MONTH",
      "TOTAL_QUANTITY",
      TRUE AS "menura.present"
    FROM "menura_2_aggregate"
  ),
  "menura_6_present" AS (
    SELECT
      "REGION",
      "CREATED_MONTH",
      "TOTAL_AMOUNT",
      TRUE AS "menura.present"
    FROM "menura_4_aggregate"
  ),
  "menura_7_keys" AS (
    SELECT
      "REGION",
      "CREATED_MONTH"
    FROM "menura_2_aggregate"
    UNION
    SELECT
      "REGION",
      "CREATED_MONTH"
    FROM "menura_4_aggregate"
  ),
  "menura_8_stitch" AS (
    SELECT
      "menura_keys"."REGION" AS "REGION",
      "menura_keys"."CREATED_MONTH" AS "CREATED_MONTH",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."TOTAL_QUANTITY" ELSE NULL END AS "TOTAL_QUANTITY",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."TOTAL_AMOUNT" ELSE NULL END AS "TOTAL_AMOUNT"
    FROM "menura_7_keys" AS "menura_keys"
    LEFT JOIN "menura_5_present" AS "menura_branch_1" ON "menura_keys"."REGION" IS NOT DISTINCT FROM "menura_branch_1"."REGION" AND "menura_keys"."CREATED_MONTH" IS NOT DISTINCT FROM "menura_branch_1"."CREATED_MONTH"
    LEFT JOIN "menura_6_present" AS "menura_branch_2" ON "menura_keys"."REGION" IS NOT DISTINCT FROM "menura_branch_2"."REGION" AND "menura_keys"."CREATED_MONTH" IS NOT DISTINCT FROM "menura_branch_2"."CREATED_MONTH"
  )
SELECT
  "REGION",
  "CREATED_MONTH",
  "TOTAL_QUANTITY",
  "TOTAL_AMOUNT"
FROM "menura_8_stitch"