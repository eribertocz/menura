WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "statement_month" AS "STATEMENT_MONTH"
    FROM "main"."statements"
  ),
  "menura_2_scan" AS (
    SELECT
      "period" AS "PERIOD"
    FROM "main"."months"
  ),
  "menura_3_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."PERIOD" AS "PERIOD"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_2_scan" AS "menura_period" ON CAST(date_trunc('month', "menura_row"."STATEMENT_MONTH") AS DATE) = "menura_period"."PERIOD"
    WHERE "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  )
SELECT
  "CUSTOMER_ID",
  "PERIOD"
FROM "menura_3_population"