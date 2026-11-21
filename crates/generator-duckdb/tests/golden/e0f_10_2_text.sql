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
      CAST(CAST("REGION" || CAST($1 AS VARCHAR) || "REGION" AS VARCHAR) AS VARCHAR) AS "JOINED",
      CAST(CAST(CASE WHEN "REGION" IS NULL OR CAST($2 AS BIGINT) IS NULL OR CAST($3 AS BIGINT) IS NULL OR CAST($3 AS BIGINT) < 0 THEN NULL ELSE substring("REGION", CAST(least(greatest(CAST($2 AS BIGINT), 1), length("REGION") + 1) AS BIGINT), CAST(least(greatest(CAST(CAST($2 AS BIGINT) AS HUGEINT) + CAST($3 AS BIGINT) - greatest(CAST($2 AS BIGINT), 1), 0), length("REGION")) AS BIGINT)) END AS VARCHAR) AS VARCHAR) AS "PART",
      CAST(CAST(CASE WHEN CAST($4 AS VARCHAR) = '' AND "REGION" IS NOT NULL THEN 1 ELSE strpos("REGION", CAST($4 AS VARCHAR)) END AS BIGINT) AS BIGINT) AS "FOUND",
      CAST("REGION" LIKE CAST($5 AS VARCHAR) AS BOOLEAN) AS "MATCHES"
    FROM "menura_1_scan"
  )
SELECT
  "JOINED",
  "PART",
  "FOUND",
  "MATCHES"
FROM "menura_2_project"