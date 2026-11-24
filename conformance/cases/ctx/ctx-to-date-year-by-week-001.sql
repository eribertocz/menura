WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      CAST(date_trunc('week', "ORDER_DATE") AS DATE) AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_4_todate" AS (
    SELECT
      "menura_rows"."AMOUNT" AS "AMOUNT",
      "menura_rows"."ORDER_DATE" AS "ORDER_DATE",
      CAST("menura_series"."menura.start" AS DATE) AS "menura.period"
    FROM "menura_3_scan" AS "menura_rows"
    CROSS JOIN generate_series(CAST(date_trunc('week', "menura_rows"."ORDER_DATE") AS TIMESTAMP), CAST(date_trunc('week', "menura_rows"."ORDER_DATE") AS TIMESTAMP) + INTERVAL 1 YEAR, INTERVAL 7 DAY) AS "menura_series"("menura.start")
    WHERE "menura_rows"."ORDER_DATE" IS NOT NULL AND isoyear("menura_series"."menura.start") = isoyear(CAST(date_trunc('week', "menura_rows"."ORDER_DATE") AS TIMESTAMP))
  ),
  "menura_5_aggregate" AS (
    SELECT
      CAST(date_trunc('week', "menura.period") AS DATE) AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_4_todate"
    GROUP BY 1
  ),
  "menura_6_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_5_aggregate"
  ),
  "menura_7_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_2_aggregate" AS "menura_result"
    LEFT JOIN "menura_6_present" AS "menura_context" ON "menura_result"."dimension_1" IS NOT DISTINCT FROM "menura_context"."dimension_1"
  ),
  "menura_8_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "sales",
      "context_1_component_1" AS "sales_ytd"
    FROM "menura_7_lookup"
  )
SELECT
  "order_date",
  "sales",
  "sales_ytd"
FROM "menura_8_project"