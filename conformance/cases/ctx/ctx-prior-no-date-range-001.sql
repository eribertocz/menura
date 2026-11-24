WITH
  "menura_1_scan" AS (
    SELECT
      "order_date" AS "ORDER_DATE",
      "order_id" AS "ORDER_ID"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "ORDER_DATE") AS DATE) AS "dimension_1",
      CAST(COUNT("ORDER_ID") AS BIGINT) AS "metric_1_component_1",
      CAST(COUNT("ORDER_ID") AS BIGINT) AS "metric_2_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST(CAST("dimension_1" + to_years(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) AS DATE) AS "context_1_shifted_time",
      CAST(TRUE AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_2_aggregate"
  ),
  "menura_4_scan" AS (
    SELECT
      "order_date" AS "ORDER_DATE",
      "order_id" AS "ORDER_ID"
    FROM "shop"."orders"
  ),
  "menura_5_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "ORDER_DATE") AS DATE) AS "dimension_1",
      CAST(COUNT("ORDER_ID") AS BIGINT) AS "metric_1_component_1"
    FROM "menura_4_scan"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      CAST("dimension_1" IS NOT NULL AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_5_aggregate"
  ),
  "menura_7_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "context_1_time_present",
      TRUE AS "menura.present"
    FROM "menura_6_project"
  ),
  "menura_8_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE CAST(0 AS BIGINT) END AS "context_1_component_1"
    FROM "menura_3_project" AS "menura_result"
    LEFT JOIN "menura_7_present" AS "menura_context" ON "menura_result"."context_1_shifted_time" IS NOT DISTINCT FROM "menura_context"."dimension_1" AND "menura_result"."context_1_time_present" IS NOT DISTINCT FROM "menura_context"."context_1_time_present"
  ),
  "menura_9_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "order_count",
      "context_1_component_1" AS "order_count_prior_year"
    FROM "menura_8_lookup"
  )
SELECT
  "order_date",
  "order_count",
  "order_count_prior_year"
FROM "menura_9_project"