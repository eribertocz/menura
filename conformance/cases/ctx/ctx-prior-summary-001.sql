WITH
  "menura_1_scan" AS (
    SELECT
      "amount_sum" AS "AMOUNT_SUM",
      "order_month" AS "ORDER_MONTH"
    FROM "shop"."orders_by_region_month"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH"
    FROM "menura_1_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_3_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH"
    FROM "menura_2_filter"
    WHERE ("ORDER_MONTH" >= CAST($1 AS DATE)) AND ("ORDER_MONTH" <= CAST($2 AS DATE))
  ),
  "menura_4_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "ORDER_MONTH") AS DATE) AS "dimension_1",
      CAST(SUM("AMOUNT_SUM") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT_SUM") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_3_filter"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST(CAST("dimension_1" + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) AS DATE) AS "context_1_shifted_time",
      CAST(TRUE AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_4_aggregate"
  ),
  "menura_6_scan" AS (
    SELECT
      "amount_sum" AS "AMOUNT_SUM",
      "order_month" AS "ORDER_MONTH"
    FROM "shop"."orders_by_region_month"
  ),
  "menura_7_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH"
    FROM "menura_6_scan"
    WHERE TRUE AND TRUE
  ),
  "menura_8_filter" AS (
    SELECT
      "AMOUNT_SUM",
      "ORDER_MONTH"
    FROM "menura_7_filter"
    WHERE (("ORDER_MONTH" >= CAST(CAST(date_trunc('month', CAST($1 AS DATE)) AS DATE) + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE)) AND ("ORDER_MONTH" < CAST(CAST(CAST(date_trunc('month', CAST($2 AS DATE)) AS DATE) + to_months(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) + to_months(CAST(1 AS INTEGER)) AS DATE)))
  ),
  "menura_9_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "ORDER_MONTH") AS DATE) AS "dimension_1",
      CAST(SUM("AMOUNT_SUM") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_8_filter"
    GROUP BY 1
  ),
  "menura_10_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_9_aggregate"
  ),
  "menura_11_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_1_time_present",
      TRUE AS "menura.present"
    FROM "menura_10_project"
  ),
  "menura_12_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_5_project" AS "menura_result"
    LEFT JOIN "menura_11_present" AS "menura_context" ON "menura_result"."context_1_shifted_time" IS NOT DISTINCT FROM "menura_context"."dimension_1" AND "menura_result"."context_1_time_present" IS NOT DISTINCT FROM "menura_context"."context_1_time_present"
  ),
  "menura_13_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "total_amount",
      "context_1_component_1" AS "total_amount_prior_month"
    FROM "menura_12_lookup"
  )
SELECT
  "order_date",
  "total_amount",
  "total_amount_prior_month"
FROM "menura_13_project"