WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "ordered_at" AS "ORDERED_AT"
    FROM "shop"."main"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "ORDERED_AT"
    FROM "menura_1_scan"
    WHERE ("ORDERED_AT" >= CAST(CAST($1 AS DATE) AS TIMESTAMP) AT TIME ZONE CAST($2 AS VARCHAR)) AND ("ORDERED_AT" < CAST(CAST($3 AS DATE) AS TIMESTAMP) AT TIME ZONE CAST($2 AS VARCHAR))
  ),
  "menura_3_aggregate" AS (
    SELECT
      date_trunc('quarter', "ORDERED_AT" AT TIME ZONE CAST($2 AS VARCHAR)) AT TIME ZONE CAST($2 AS VARCHAR) AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_project" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      "metric_1_component_1" AS "metric_1_component_1",
      "metric_2_component_1" AS "metric_2_component_1",
      CAST(CAST((("dimension_1" AT TIME ZONE CAST($2 AS VARCHAR)) + to_years(CAST(-CAST(1 AS HUGEINT) AS INTEGER))) AT TIME ZONE CAST($2 AS VARCHAR) AS TIMESTAMP WITH TIME ZONE) AS TIMESTAMP WITH TIME ZONE) AS "context_1_shifted_time",
      CAST(TRUE AS BOOLEAN) AS "context_1_time_present"
    FROM "menura_3_aggregate"
  ),
  "menura_5_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "ordered_at" AS "ORDERED_AT"
    FROM "shop"."main"."orders"
  ),
  "menura_6_filter" AS (
    SELECT
      "AMOUNT",
      "ORDERED_AT"
    FROM "menura_5_scan"
    WHERE (("ORDERED_AT" >= CAST(CAST(CAST(date_trunc('quarter', CAST($1 AS DATE)) AS DATE) + to_years(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) AS TIMESTAMP) AT TIME ZONE CAST($2 AS VARCHAR)) AND ("ORDERED_AT" < CAST(CAST(CAST(CAST(date_trunc('quarter', CAST($4 AS DATE)) AS DATE) + to_years(CAST(-CAST(1 AS HUGEINT) AS INTEGER)) AS DATE) + to_months(CAST(1 AS INTEGER) * 3) AS DATE) AS TIMESTAMP) AT TIME ZONE CAST($2 AS VARCHAR)))
  ),
  "menura_7_aggregate" AS (
    SELECT
      date_trunc('quarter', "ORDERED_AT" AT TIME ZONE CAST($2 AS VARCHAR)) AT TIME ZONE CAST($2 AS VARCHAR) AS "dimension_1",
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
      "menura_result"."context_1_shifted_time" AS "context_1_shifted_time",
      "menura_result"."context_1_time_present" AS "context_1_time_present",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_4_project" AS "menura_result"
    LEFT JOIN "menura_9_present" AS "menura_context" ON "menura_result"."context_1_shifted_time" IS NOT DISTINCT FROM "menura_context"."dimension_1" AND "menura_result"."context_1_time_present" IS NOT DISTINCT FROM "menura_context"."context_1_time_present"
  ),
  "menura_11_project" AS (
    SELECT
      "dimension_1" AS "ordered_at",
      "metric_1_component_1" AS "sales",
      "context_1_component_1" AS "sales_prior_year_at"
    FROM "menura_10_lookup"
  )
SELECT
  "ordered_at",
  "sales",
  "sales_prior_year_at"
FROM "menura_11_project"