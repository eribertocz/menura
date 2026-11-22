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
    WHERE "menura_row"."CUSTOMER_ID" IS NOT NULL
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
      "menura_value"."BALANCE" AS "BALANCE"
    FROM "menura_5_values" AS "menura_value"
    LEFT JOIN "menura_4_population" AS "menura_expected" ON "menura_value"."CUSTOMER_ID" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."PERIOD"
  ),
  "menura_7_present" AS (
    SELECT
      "CUSTOMER_ID",
      "menura.period"
    FROM "menura_6_checked"
    GROUP BY 1, 2
  ),
  "menura_8_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_expected"."PERIOD" AS "menura.period"
    FROM "menura_4_population" AS "menura_expected"
    LEFT JOIN "menura_7_present" AS "menura_present" ON "menura_present"."CUSTOMER_ID" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.period" = "menura_expected"."PERIOD"
    WHERE "menura_present"."CUSTOMER_ID" IS NULL
  ),
  "menura_9_selected" AS (
    SELECT
      "BRANCH",
      "BALANCE_YEAR",
      MAX("menura.period") AS "menura.period"
    FROM "menura_5_values"
    GROUP BY 1, 2
  ),
  "menura_10_violations" AS (
    SELECT
      'RADD-01: missing: error and an expected entity has no row in metric BALANCE' AS "menura.violation"
    FROM "menura_8_missing"
    UNION ALL
    SELECT
      'RADD-02: unexpected: error and an unexpected row exists in metric BALANCE' AS "menura.violation"
    FROM "menura_5_values" AS "menura_value"
    LEFT JOIN "menura_4_population" AS "menura_expected" ON "menura_value"."CUSTOMER_ID" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."PERIOD"
    WHERE "menura_expected"."PERIOD" IS NULL
    UNION ALL
    SELECT
      'RADD-03: nulls: error and a NULL value exists in metric BALANCE' AS "menura.violation"
    FROM "menura_6_checked"
    WHERE "BALANCE" IS NULL
  ),
  "menura_11_snapshot" AS (
    SELECT
      "menura_candidate"."BRANCH",
      "menura_candidate"."BALANCE_YEAR",
      "menura_candidate"."CUSTOMER_ID",
      CAST("menura_candidate"."BALANCE" AS DECIMAL(38,2)) AS "BALANCE"
    FROM "menura_6_checked" AS "menura_candidate"
    JOIN "menura_9_selected" AS "menura_selected" ON "menura_candidate"."BRANCH" IS NOT DISTINCT FROM "menura_selected"."BRANCH" AND "menura_candidate"."BALANCE_YEAR" IS NOT DISTINCT FROM "menura_selected"."BALANCE_YEAR" AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
    UNION ALL
    SELECT
      NULL AS "BRANCH",
      NULL AS "BALANCE_YEAR",
      NULL AS "CUSTOMER_ID",
      error(MIN("menura.violation")) AS "BALANCE"
    FROM "menura_10_violations"
    HAVING MIN("menura.violation") IS NOT NULL
  )
SELECT
  "BRANCH",
  "BALANCE_YEAR",
  "CUSTOMER_ID",
  "BALANCE"
FROM "menura_11_snapshot"