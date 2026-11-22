WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "balance_date" AS "BALANCE_DATE",
      "branch" AS "BRANCH",
      "balance" AS "BALANCE"
    FROM "main"."balances"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "main"."customers"
  ),
  "menura_3_scan" AS (
    SELECT
      "period" AS "PERIOD"
    FROM "main"."months"
  ),
  "menura_4_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."PERIOD" AS "PERIOD"
    FROM "menura_2_scan" AS "menura_row"
    JOIN "menura_3_scan" AS "menura_period" ON "menura_row"."VALID_FROM" <= CAST("menura_period"."PERIOD" + INTERVAL 1 MONTH - INTERVAL 1 DAY AS DATE) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."PERIOD")
    GROUP BY 1, 2
  ),
  "menura_5_values" AS (
    SELECT
      "BRANCH" AS "BRANCH",
      CAST(date_trunc('year', "BALANCE_DATE") AS DATE) AS "BALANCE_YEAR",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      CAST(date_trunc('month', "BALANCE_DATE") AS DATE) AS "menura.period",
      SUM("BALANCE") AS "BALANCE"
    FROM "menura_1_scan"
    GROUP BY 1, 2, 3, 4
  ),
  "menura_6_checked" AS (
    SELECT
      "menura_value"."BRANCH",
      "menura_value"."BALANCE_YEAR",
      "menura_value"."CUSTOMER_ID",
      "menura_value"."menura.period",
      COALESCE("menura_value"."BALANCE", 0) AS "BALANCE"
    FROM "menura_5_values" AS "menura_value"
  ),
  "menura_7_selected" AS (
    SELECT
      "BRANCH",
      "BALANCE_YEAR",
      MAX("menura.period") AS "menura.period"
    FROM "menura_5_values"
    GROUP BY 1, 2
  ),
  "menura_8_snapshot" AS (
    SELECT
      "menura_candidate"."BRANCH",
      "menura_candidate"."BALANCE_YEAR",
      "menura_candidate"."CUSTOMER_ID",
      CAST("menura_candidate"."BALANCE" AS DECIMAL(38,2)) AS "BALANCE"
    FROM "menura_6_checked" AS "menura_candidate"
    JOIN "menura_7_selected" AS "menura_selected" ON "menura_candidate"."BRANCH" IS NOT DISTINCT FROM "menura_selected"."BRANCH" AND "menura_candidate"."BALANCE_YEAR" IS NOT DISTINCT FROM "menura_selected"."BALANCE_YEAR" AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  )
SELECT
  "BRANCH",
  "BALANCE_YEAR",
  "CUSTOMER_ID",
  "BALANCE"
FROM "menura_8_snapshot"