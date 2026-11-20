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
      CAST(CAST("REGION" || $1 || "REGION" AS VARCHAR) AS VARCHAR) AS "JOINED",
      CAST(CAST(CASE WHEN "REGION" IS NULL OR $2 IS NULL OR $3 IS NULL OR $3 < 0 THEN NULL ELSE substring("REGION", CAST(least(greatest($2, 1), length("REGION") + 1) AS BIGINT), CAST(least(greatest(CAST($2 AS HUGEINT) + $3 - greatest($2, 1), 0), length("REGION")) AS BIGINT)) END AS VARCHAR) AS VARCHAR) AS "PART",
      CAST(CAST(CASE WHEN $4 = '' AND "REGION" IS NOT NULL THEN 1 ELSE strpos("REGION", $4) END AS BIGINT) AS BIGINT) AS "FOUND",
      CAST("REGION" LIKE $5 AS BOOLEAN) AS "MATCHES"
    FROM "menura_1_scan"
  )
SELECT
  "JOINED",
  "PART",
  "FOUND",
  "MATCHES"
FROM "menura_2_project"