WITH
  "menura_1_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_2_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD"
    FROM "menura_1_scan"
    WHERE ("PERIOD" >= CAST($1 AS DATE)) AND ("PERIOD" <= CAST($2 AS DATE))
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_4_periods" AS (
    SELECT
      CAST("menura.start" AS DATE) AS "period"
    FROM generate_series(CAST(date_trunc('month', CAST($1 AS DATE)) AS TIMESTAMP), CAST(CAST($2 AS DATE) AS TIMESTAMP), INTERVAL 1 MONTH) AS "menura_periods"("menura.start")
  ),
  "menura_5_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_3_scan" AS "menura_row"
    JOIN "menura_4_periods" AS "menura_period" ON "menura_row"."VALID_FROM" <= CAST("menura_period"."period" + INTERVAL 1 MONTH - INTERVAL 1 DAY AS DATE) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")
    WHERE "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_6_values" AS (
    SELECT
      CAST(date_trunc('month', "PERIOD") AS DATE) AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      CAST(date_trunc('month', "PERIOD") AS DATE) AS "menura.period",
      SUM("BALANCE") AS "metric_1_component_1",
      SUM("BALANCE") AS "metric_2_component_1"
    FROM "menura_2_filter"
    GROUP BY 1, 2, 3
  ),
  "menura_7_checked" AS (
    SELECT
      "menura_value"."dimension_1",
      "menura_value"."entity_1",
      "menura_value"."menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1",
      "menura_value"."metric_2_component_1" AS "metric_2_component_1"
    FROM "menura_6_values" AS "menura_value"
    LEFT JOIN "menura_5_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_8_snapshot" AS (
    SELECT
      "dimension_1",
      "entity_1",
      CAST(AVG("metric_1_component_1") AS HUGEINT) AS "metric_1_component_1",
      CAST(AVG("metric_2_component_1") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_7_checked"
    GROUP BY 1, 2
  ),
  "menura_9_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      CAST(SUM("metric_1_component_1") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("metric_2_component_1") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_8_snapshot"
    GROUP BY 1
  ),
  "menura_10_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST(CAST("dimension_1" + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) AS DATE) AS "context_1_shifted_time",
      CAST(TRUE AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_9_aggregate"
  ),
  "menura_11_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD"
    FROM "shop"."balances"
  ),
  "menura_12_filter" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD"
    FROM "menura_11_scan"
    WHERE (("PERIOD" >= CAST(CAST(date_trunc('month', CAST($1 AS DATE)) AS DATE) + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE)) AND ("PERIOD" < CAST(CAST(CAST(date_trunc('month', CAST($2 AS DATE)) AS DATE) + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) + to_months(CAST(1 AS INTEGER)) AS DATE)))
  ),
  "menura_13_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."members"
  ),
  "menura_14_periods" AS (
    SELECT
      CAST("menura.start" AS DATE) AS "period"
    FROM generate_series(CAST(date_trunc('month', CAST(CAST(date_trunc('month', CAST($1 AS DATE)) AS DATE) + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE)) AS TIMESTAMP), CAST(CAST(CAST(CAST(CAST(date_trunc('month', CAST($2 AS DATE)) AS DATE) + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) + to_months(CAST(1 AS INTEGER)) AS DATE) + to_days(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) AS TIMESTAMP), INTERVAL 1 MONTH) AS "menura_periods"("menura.start")
  ),
  "menura_15_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_13_scan" AS "menura_row"
    JOIN "menura_14_periods" AS "menura_period" ON "menura_row"."VALID_FROM" <= CAST("menura_period"."period" + INTERVAL 1 MONTH - INTERVAL 1 DAY AS DATE) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")
    WHERE "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_16_values" AS (
    SELECT
      CAST(date_trunc('month', "PERIOD") AS DATE) AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      CAST(date_trunc('month', "PERIOD") AS DATE) AS "menura.period",
      SUM("BALANCE") AS "metric_1_component_1"
    FROM "menura_12_filter"
    GROUP BY 1, 2, 3
  ),
  "menura_17_checked" AS (
    SELECT
      "menura_value"."dimension_1",
      "menura_value"."entity_1",
      "menura_value"."menura.period",
      "menura_value"."metric_1_component_1" AS "metric_1_component_1"
    FROM "menura_16_values" AS "menura_value"
    LEFT JOIN "menura_15_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_18_snapshot" AS (
    SELECT
      "dimension_1",
      "entity_1",
      CAST(AVG("metric_1_component_1") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_17_checked"
    GROUP BY 1, 2
  ),
  "menura_19_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      CAST(SUM("metric_1_component_1") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_18_snapshot"
    GROUP BY 1
  ),
  "menura_20_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_19_aggregate"
  ),
  "menura_21_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_1_time_present",
      TRUE AS "menura.present"
    FROM "menura_20_project"
  ),
  "menura_22_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_10_project" AS "menura_result"
    LEFT JOIN "menura_21_present" AS "menura_context" ON "menura_result"."context_1_shifted_time" IS NOT DISTINCT FROM "menura_context"."dimension_1" AND "menura_result"."context_1_time_present" IS NOT DISTINCT FROM "menura_context"."context_1_time_present"
  ),
  "menura_23_project" AS (
    SELECT
      "dimension_1" AS "period",
      "metric_1_component_1" AS "average_balance",
      "context_1_component_1" AS "average_balance_prior_month"
    FROM "menura_22_lookup"
  )
SELECT
  "period",
  "average_balance",
  "average_balance_prior_month"
FROM "menura_23_project"