WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_1_scan"
    WHERE ("ORDER_DATE" >= CAST($1 AS DATE)) AND ("ORDER_DATE" <= CAST($2 AS DATE))
  ),
  "menura_3_aggregate" AS (
    SELECT
      "ORDER_DATE" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_2_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_3_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      "metric_3_component_1" AS "metric_3_component_1",
      CAST(CAST("dimension_1" + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) AS DATE) AS "context_1_shifted_time",
      CAST(TRUE AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_3_aggregate"
  ),
  "menura_5_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_6_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_5_scan"
    WHERE (("ORDER_DATE" >= CAST(CAST($1 AS DATE) + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE)) AND ("ORDER_DATE" < CAST(CAST(CAST($2 AS DATE) + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) + to_days(CAST(1 AS INTEGER)) AS DATE)))
  ),
  "menura_7_aggregate" AS (
    SELECT
      "ORDER_DATE" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_6_filter"
    GROUP BY 1
  ),
  "menura_8_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_7_aggregate"
  ),
  "menura_9_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_1_time_present",
      TRUE AS "menura.present"
    FROM "menura_8_project"
  ),
  "menura_10_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."metric_3_component_1" AS "metric_3_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_4_project" AS "menura_result"
    LEFT JOIN "menura_9_present" AS "menura_context" ON "menura_result"."context_1_shifted_time" IS NOT DISTINCT FROM "menura_context"."dimension_1" AND "menura_result"."context_1_time_present" IS NOT DISTINCT FROM "menura_context"."context_1_time_present"
  ),
  "menura_11_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      "metric_3_component_1" AS "metric_3_component_1",
      "context_1_shifted_time" AS "context_1_shifted_time",
      "context_1_time_present" AS "context_1_time_present",
      "context_1_component_1" AS "context_1_component_1",
      CAST(CAST("dimension_1" + to_years(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) AS DATE) AS "context_2_shifted_time",
      CAST(TRUE AS BOOLEAN) AS "context_2_time_present"
    FROM "menura_10_lookup"
  ),
  "menura_12_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_13_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_12_scan"
    WHERE (("ORDER_DATE" >= CAST(CAST($1 AS DATE) + to_years(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE)) AND ("ORDER_DATE" < CAST(CAST(CAST($2 AS DATE) + to_years(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) + to_days(CAST(1 AS INTEGER)) AS DATE)))
  ),
  "menura_14_aggregate" AS (
    SELECT
      "ORDER_DATE" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_13_filter"
    GROUP BY 1
  ),
  "menura_15_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS BOOLEAN) AS "context_2_time_present"
    FROM "menura_14_aggregate"
  ),
  "menura_16_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_2_time_present",
      TRUE AS "menura.present"
    FROM "menura_15_project"
  ),
  "menura_17_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."metric_3_component_1" AS "metric_3_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      "menura_result"."context_1_component_1" AS "context_1_component_1",
      "menura_result"."context_2_shifted_time" AS "context_2_shifted_time",
      "menura_result"."context_2_time_present" AS "context_2_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_2_component_1"
    FROM "menura_11_project" AS "menura_result"
    LEFT JOIN "menura_16_present" AS "menura_context" ON "menura_result"."context_2_shifted_time" IS NOT DISTINCT FROM "menura_context"."dimension_1" AND "menura_result"."context_2_time_present" IS NOT DISTINCT FROM "menura_context"."context_2_time_present"
  ),
  "menura_18_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "sales",
      "context_1_component_1" AS "sales_prior_month",
      "context_2_component_1" AS "sales_prior_year"
    FROM "menura_17_lookup"
  )
SELECT
  "order_date",
  "sales",
  "sales_prior_month",
  "sales_prior_year"
FROM "menura_18_project"