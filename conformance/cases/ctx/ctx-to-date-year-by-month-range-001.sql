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
      CAST(date_trunc('month', "ORDER_DATE") AS DATE) AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_5_todate" AS (
    SELECT
      "menura_rows"."AMOUNT" AS "AMOUNT",
      "menura_rows"."ORDER_DATE" AS "ORDER_DATE",
      CAST("menura_series"."menura.start" AS DATE) AS "menura.period"
    FROM "menura_4_scan" AS "menura_rows"
    CROSS JOIN generate_series(CAST(date_trunc('month', "menura_rows"."ORDER_DATE") AS TIMESTAMP), CAST(date_trunc('month', "menura_rows"."ORDER_DATE") AS TIMESTAMP) + INTERVAL 1 YEAR, INTERVAL 1 MONTH) AS "menura_series"("menura.start")
    WHERE "menura_rows"."ORDER_DATE" IS NOT NULL AND date_trunc('year', "menura_series"."menura.start") = date_trunc('year', CAST(date_trunc('month', "menura_rows"."ORDER_DATE") AS TIMESTAMP)) AND CAST("menura_series"."menura.start" AS DATE) <= CAST($4 AS DATE) AND CAST("menura_series"."menura.start" + INTERVAL 1 MONTH AS DATE) > CAST($3 AS DATE)
  ),
  "menura_6_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "menura.period") AS DATE) AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_5_todate"
    GROUP BY 1
  ),
  "menura_7_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_6_aggregate"
  ),
  "menura_8_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_3_aggregate" AS "menura_result"
    LEFT JOIN "menura_7_present" AS "menura_context" ON "menura_result"."dimension_1" IS NOT DISTINCT FROM "menura_context"."dimension_1"
  ),
  "menura_9_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "sales",
      "context_1_component_1" AS "sales_ytd"
    FROM "menura_8_lookup"
  )
SELECT
  "order_date",
  "sales",
  "sales_ytd"
FROM "menura_9_project"