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
  "menura_5_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "balance_date" AS "BALANCE_DATE",
      "branch" AS "BRANCH",
      "balance" AS "BALANCE"
    FROM "main"."added_balances"
  ),
  "menura_6_values" AS (
    SELECT
      "BRANCH" AS "BRANCH",
      CAST(date_trunc('year', "BALANCE_DATE") AS DATE) AS "BALANCE_YEAR",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      CAST(date_trunc('month', "BALANCE_DATE") AS DATE) AS "menura.period",
      SUM("BALANCE") AS "BALANCE"
    FROM "menura_1_scan"
    GROUP BY 1, 2, 3, 4
  ),
  "menura_7_checked" AS (
    SELECT
      "menura_value"."BRANCH",
      "menura_value"."BALANCE_YEAR",
      "menura_value"."CUSTOMER_ID",
      "menura_value"."menura.period",
      "menura_value"."BALANCE" AS "BALANCE"
    FROM "menura_6_values" AS "menura_value"
  ),
  "menura_8_present" AS (
    SELECT
      "CUSTOMER_ID",
      "menura.period"
    FROM "menura_7_checked"
    GROUP BY 1, 2
  ),
  "menura_9_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_expected"."PERIOD" AS "menura.period"
    FROM "menura_4_population" AS "menura_expected"
    LEFT JOIN "menura_8_present" AS "menura_present" ON "menura_present"."CUSTOMER_ID" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.period" = "menura_expected"."PERIOD"
    WHERE "menura_present"."CUSTOMER_ID" IS NULL
  ),
  "menura_10_added" AS (
    SELECT
      "BRANCH" AS "BRANCH",
      CAST(date_trunc('year', "BALANCE_DATE") AS DATE) AS "BALANCE_YEAR",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      CAST(date_trunc('month', "BALANCE_DATE") AS DATE) AS "menura.period"
    FROM "menura_5_scan"
  ),
  "menura_11_zero" AS (
    SELECT
      "menura_fill"."BRANCH",
      "menura_fill"."BALANCE_YEAR",
      "menura_fill"."CUSTOMER_ID",
      "menura_fill"."menura.period",
      CAST(0 AS DECIMAL(38,2)) AS "BALANCE"
    FROM "menura_10_added" AS "menura_fill"
    JOIN "menura_9_missing" AS "menura_missing" ON "menura_fill"."CUSTOMER_ID" = "menura_missing"."CUSTOMER_ID" AND "menura_fill"."menura.period" = "menura_missing"."menura.period"
  ),
  "menura_12_candidates" AS (
    SELECT
      "BRANCH",
      "BALANCE_YEAR",
      "CUSTOMER_ID",
      "menura.period",
      "BALANCE"
    FROM "menura_7_checked"
    UNION ALL
    SELECT
      "BRANCH",
      "BALANCE_YEAR",
      "CUSTOMER_ID",
      "menura.period",
      "BALANCE"
    FROM "menura_11_zero"
  ),
  "menura_13_selected" AS (
    SELECT
      "BRANCH",
      "BALANCE_YEAR",
      MAX("menura.period") AS "menura.period"
    FROM "menura_6_values"
    GROUP BY 1, 2
  ),
  "menura_14_snapshot" AS (
    SELECT
      "menura_candidate"."BRANCH",
      "menura_candidate"."BALANCE_YEAR",
      "menura_candidate"."CUSTOMER_ID",
      CAST("menura_candidate"."BALANCE" AS DECIMAL(38,2)) AS "BALANCE"
    FROM "menura_12_candidates" AS "menura_candidate"
    JOIN "menura_13_selected" AS "menura_selected" ON "menura_candidate"."BRANCH" IS NOT DISTINCT FROM "menura_selected"."BRANCH" AND "menura_candidate"."BALANCE_YEAR" IS NOT DISTINCT FROM "menura_selected"."BALANCE_YEAR" AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  )
SELECT
  "BRANCH",
  "BALANCE_YEAR",
  "CUSTOMER_ID",
  "BALANCE"
FROM "menura_14_snapshot"