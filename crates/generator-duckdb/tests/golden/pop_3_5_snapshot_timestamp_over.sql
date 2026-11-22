WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "balance_date" AS "BALANCE_DATE",
      "branch" AS "BRANCH",
      "balance" AS "BALANCE",
      "booked_at" AS "BOOKED_AT"
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
    FROM "main"."instants"
  ),
  "menura_4_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."PERIOD" AS "PERIOD"
    FROM "menura_2_scan" AS "menura_row"
    JOIN "menura_3_scan" AS "menura_period" ON "menura_row"."VALID_FROM" <= CAST(CAST(("menura_period"."PERIOD") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE) + INTERVAL 1 MONTH - INTERVAL 1 DAY AS DATE) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= CAST(("menura_period"."PERIOD") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE))
    WHERE "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_5_values" AS (
    SELECT
      "BRANCH" AS "BRANCH",
      "CUSTOMER_ID" AS "CUSTOMER_ID",
      CAST((date_trunc('month', "BOOKED_AT" AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR)) AT TIME ZONE CAST($1 AS VARCHAR) AS DATE) AS "menura.period",
      SUM("BALANCE") AS "BALANCE"
    FROM "menura_1_scan"
    GROUP BY 1, 2, 3
  ),
  "menura_6_checked" AS (
    SELECT
      "menura_value"."BRANCH",
      "menura_value"."CUSTOMER_ID",
      "menura_value"."menura.period",
      "menura_value"."BALANCE" AS "BALANCE"
    FROM "menura_5_values" AS "menura_value"
    LEFT JOIN "menura_4_population" AS "menura_expected" ON "menura_value"."CUSTOMER_ID" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = CAST(("menura_expected"."PERIOD") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE)
    WHERE "menura_expected"."PERIOD" IS NOT NULL
  ),
  "menura_7_present" AS (
    SELECT
      "CUSTOMER_ID"
    FROM "menura_6_checked"
    GROUP BY 1
  ),
  "menura_8_edges" AS (
    SELECT
      MAX(CAST(("PERIOD") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE)) AS "menura.period"
    FROM "menura_3_scan"
  ),
  "menura_9_missing" AS (
    SELECT
      "menura_expected"."CUSTOMER_ID" AS "CUSTOMER_ID",
      CAST(("menura_expected"."PERIOD") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE) AS "menura.period"
    FROM "menura_4_population" AS "menura_expected"
    JOIN "menura_8_edges" AS "menura_edge" ON CAST(("menura_expected"."PERIOD") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE) = "menura_edge"."menura.period"
    LEFT JOIN "menura_7_present" AS "menura_present" ON "menura_present"."CUSTOMER_ID" = "menura_expected"."CUSTOMER_ID"
    WHERE "menura_present"."CUSTOMER_ID" IS NULL
  ),
  "menura_10_violations" AS (
    SELECT
      'RADD-01: missing: error and an expected entity has no row in metric BALANCE' AS "menura.violation"
    FROM "menura_9_missing"
  ),
  "menura_11_snapshot" AS (
    SELECT
      "BRANCH",
      "CUSTOMER_ID",
      CAST("BALANCE" AS DECIMAL(38,2)) AS "BALANCE"
    FROM "menura_6_checked"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BRANCH", "CUSTOMER_ID" ORDER BY "menura.period" DESC NULLS LAST) = 1
    UNION ALL
    SELECT
      NULL AS "BRANCH",
      NULL AS "CUSTOMER_ID",
      error(MIN("menura.violation")) AS "BALANCE"
    FROM "menura_10_violations"
    HAVING MIN("menura.violation") IS NOT NULL
  )
SELECT
  "BRANCH",
  "CUSTOMER_ID",
  "BALANCE"
FROM "menura_11_snapshot"