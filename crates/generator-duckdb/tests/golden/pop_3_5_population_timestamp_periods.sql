WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO",
      "is_billable" AS "IS_BILLABLE"
    FROM "main"."customers"
  ),
  "menura_2_scan" AS (
    SELECT
      "period" AS "PERIOD"
    FROM "main"."instants"
  ),
  "menura_3_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."PERIOD" AS "PERIOD"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_2_scan" AS "menura_period" ON "menura_row"."VALID_FROM" <= CAST(("menura_period"."PERIOD") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= CAST(CAST(("menura_period"."PERIOD") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE) + INTERVAL 1 MONTH - INTERVAL 1 DAY AS DATE))
    WHERE "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  )
SELECT
  "CUSTOMER_ID",
  "PERIOD"
FROM "menura_3_population"