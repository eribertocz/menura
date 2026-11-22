WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."main"."orders"
  ),
  "menura_2_project" AS (
    SELECT
      "AMOUNT" AS "AMOUNT",
      "ORDER_DATE" AS "ORDER_DATE",
      CAST(CAST(date_trunc('month', "ORDER_DATE") AS DATE) AS DATE) AS "menura.dimension_1"
    FROM "menura_1_scan"
  ),
  "menura_3_aggregate" AS (
    SELECT
      "menura.dimension_1" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_2_project"
    GROUP BY 1
  ),
  "menura_4_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."main"."orders"
  ),
  "menura_5_project" AS (
    SELECT
      "AMOUNT" AS "AMOUNT",
      "ORDER_DATE" AS "ORDER_DATE",
      CAST(CAST(date_trunc('month', "ORDER_DATE") AS DATE) AS DATE) AS "menura.dimension_1"
    FROM "menura_4_scan"
  ),
  "menura_6_todate" AS (
    SELECT
      "menura_rows"."AMOUNT" AS "AMOUNT",
      "menura_rows"."ORDER_DATE" AS "ORDER_DATE",
      "menura_rows"."menura.dimension_1" AS "menura.dimension_1",
      CAST("menura_series"."menura.start" AS DATE) AS "menura.period"
    FROM "menura_5_project" AS "menura_rows"
    CROSS JOIN generate_series(CAST(date_trunc('month', "menura_rows"."ORDER_DATE") AS TIMESTAMP), CAST(date_trunc('month', "menura_rows"."ORDER_DATE") AS TIMESTAMP) + INTERVAL 1 YEAR, INTERVAL 1 MONTH) AS "menura_series"("menura.start")
    WHERE "menura_rows"."ORDER_DATE" IS NOT NULL AND date_trunc('year', "menura_series"."menura.start") = date_trunc('year', CAST(date_trunc('month', "menura_rows"."ORDER_DATE") AS TIMESTAMP))
  ),
  "menura_7_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "menura.period") AS DATE) AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_6_todate"
    GROUP BY 1
  ),
  "menura_8_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_7_aggregate"
  ),
  "menura_9_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_3_aggregate" AS "menura_result"
    LEFT JOIN "menura_8_present" AS "menura_context" ON "menura_result"."dimension_1" IS NOT DISTINCT FROM "menura_context"."dimension_1"
  ),
  "menura_10_project" AS (
    SELECT
      "dimension_1" AS "order_month",
      "metric_1_component_1" AS "sales",
      "context_1_component_1" AS "sales_ytd"
    FROM "menura_9_lookup"
  )
SELECT
  "order_month",
  "sales",
  "sales_ytd"
FROM "menura_10_project"