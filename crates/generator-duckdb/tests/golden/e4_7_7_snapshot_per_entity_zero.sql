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
      CAST(date_trunc('year', "menura.period") AS DATE) AS "menura.unit"
    FROM "menura_7_checked"
    GROUP BY 1, 2
  ),
  "menura_9_edges" AS (
    SELECT
      CAST(date_trunc('year', "PERIOD") AS DATE) AS "menura.unit",
      MAX("PERIOD") AS "menura.period"
    FROM "menura_3_scan"
    GROUP BY 1
  ),
  "menura_10_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_expected"."PERIOD" AS "menura.period"
    FROM "menura_4_population" AS "menura_expected"
    JOIN "menura_9_edges" AS "menura_edge" ON "menura_expected"."PERIOD" = "menura_edge"."menura.period"
    LEFT JOIN "menura_8_present" AS "menura_present" ON "menura_present"."CUSTOMER_ID" = "menura_expected"."CUSTOMER_ID" AND "menura_present"."menura.unit" = "menura_edge"."menura.unit"
    WHERE "menura_present"."CUSTOMER_ID" IS NULL
  ),
  "menura_11_added" AS (
    SELECT
      "BRANCH" AS "BRANCH",
      CAST(date_trunc('year', "BALANCE_DATE") AS DATE) AS "BALANCE_YEAR",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      CAST(date_trunc('month', "BALANCE_DATE") AS DATE) AS "menura.period"
    FROM "menura_5_scan"
  ),
  "menura_12_zero" AS (
    SELECT
      "menura_fill"."BRANCH",
      "menura_fill"."BALANCE_YEAR",
      "menura_fill"."CUSTOMER_ID",
      "menura_fill"."menura.period",
      CAST(0 AS DECIMAL(38,2)) AS "BALANCE"
    FROM "menura_11_added" AS "menura_fill"
    JOIN "menura_10_missing" AS "menura_missing" ON "menura_fill"."CUSTOMER_ID" = "menura_missing"."CUSTOMER_ID" AND "menura_fill"."menura.period" = "menura_missing"."menura.period"
  ),
  "menura_13_candidates" AS (
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
    FROM "menura_12_zero"
  ),
  "menura_14_snapshot" AS (
    SELECT
      "BRANCH",
      "BALANCE_YEAR",
      "CUSTOMER_ID",
      CAST("BALANCE" AS DECIMAL(38,2)) AS "BALANCE"
    FROM "menura_13_candidates"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BRANCH", "BALANCE_YEAR", "CUSTOMER_ID" ORDER BY "menura.period" DESC NULLS LAST) = 1
  )
SELECT
  "BRANCH",
  "BALANCE_YEAR",
  "CUSTOMER_ID",
  "BALANCE"
FROM "menura_14_snapshot"