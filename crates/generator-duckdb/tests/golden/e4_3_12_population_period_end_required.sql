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
    FROM "main"."months"
  ),
  "menura_3_required" AS (
    SELECT
      "CUSTOMER_ID",
      "VALID_FROM",
      "VALID_TO",
      "IS_BILLABLE"
    FROM "menura_1_scan"
    WHERE "IS_BILLABLE"
  ),
  "menura_4_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."PERIOD" AS "PERIOD"
    FROM "menura_3_required" AS "menura_row"
    JOIN "menura_2_scan" AS "menura_period" ON "menura_row"."VALID_FROM" <= CAST("menura_period"."PERIOD" + INTERVAL 1 MONTH - INTERVAL 1 DAY AS DATE) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= CAST("menura_period"."PERIOD" + INTERVAL 1 MONTH - INTERVAL 1 DAY AS DATE))
    WHERE "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  )
SELECT
  "CUSTOMER_ID",
  "PERIOD"
FROM "menura_4_population"