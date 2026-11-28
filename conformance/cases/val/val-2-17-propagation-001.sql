WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "sale_id" AS "SALE_ID",
      "sold_on" AS "SOLD_ON"
    FROM "shop"."sales"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."customer_versions"
  ),
  "menura_3_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "segment" AS "SEGMENT",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."customer_versions"
  ),
  "menura_4_filter" AS (
    SELECT
      "CUSTOMER_ID",
      "SEGMENT",
      "VALID_FROM",
      "VALID_TO"
    FROM "menura_3_scan"
    WHERE ("SEGMENT" = 'RETAIL') AND TRUE
  ),
  "menura_5_rows" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SALE_ID",
      "SOLD_ON",
      ROW_NUMBER() OVER () AS "menura.row"
    FROM "menura_1_scan"
  ),
  "menura_6_versions" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."CUSTOMER_ID",
      "menura_row"."SALE_ID",
      "menura_row"."SOLD_ON",
      "menura_row"."menura.row",
      CASE WHEN "menura_version"."VALID_FROM" IS NOT NULL AND "menura_version"."VALID_FROM" <= "menura_row"."SOLD_ON" AND (NULLIF("menura_version"."VALID_TO", CAST('2099-12-31' AS DATE)) IS NULL OR "menura_row"."SOLD_ON" < NULLIF("menura_version"."VALID_TO", CAST('2099-12-31' AS DATE))) THEN 1 ELSE 0 END AS "menura.covers"
    FROM "menura_5_rows" AS "menura_row"
    LEFT JOIN "menura_2_scan" AS "menura_version" ON "menura_row"."CUSTOMER_ID" = "menura_version"."CUSTOMER_ID"
  ),
  "menura_7_covered" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SALE_ID",
      "SOLD_ON",
      "menura.row",
      SUM("menura.covers") OVER (PARTITION BY "menura.row") AS "menura.related",
      ROW_NUMBER() OVER (PARTITION BY "menura.row") AS "menura.rank"
    FROM "menura_6_versions"
  ),
  "menura_8_counted" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SALE_ID",
      "SOLD_ON",
      "menura.row",
      "menura.related"
    FROM "menura_7_covered"
    WHERE "menura.rank" = 1
  ),
  "menura_9_versions" AS (
    SELECT
      "menura_row"."AMOUNT",
      "menura_row"."CUSTOMER_ID",
      "menura_row"."SALE_ID",
      "menura_row"."SOLD_ON",
      "menura_row"."menura.row",
      "menura_row"."menura.related",
      CASE WHEN "menura_version"."VALID_FROM" IS NOT NULL AND "menura_version"."VALID_FROM" <= "menura_row"."SOLD_ON" AND (NULLIF("menura_version"."VALID_TO", CAST('2099-12-31' AS DATE)) IS NULL OR "menura_row"."SOLD_ON" < NULLIF("menura_version"."VALID_TO", CAST('2099-12-31' AS DATE))) THEN 1 ELSE 0 END AS "menura.covers"
    FROM "menura_8_counted" AS "menura_row"
    LEFT JOIN "menura_4_filter" AS "menura_version" ON "menura_row"."CUSTOMER_ID" = "menura_version"."CUSTOMER_ID"
  ),
  "menura_10_covered" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SALE_ID",
      "SOLD_ON",
      "menura.row",
      "menura.related",
      SUM("menura.covers") OVER (PARTITION BY "menura.row") AS "menura.visible",
      ROW_NUMBER() OVER (PARTITION BY "menura.row") AS "menura.rank"
    FROM "menura_9_versions"
  ),
  "menura_11_counted" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SALE_ID",
      "SOLD_ON",
      "menura.row",
      "menura.related",
      "menura.visible"
    FROM "menura_10_covered"
    WHERE "menura.rank" = 1
  ),
  "menura_12_semijoin" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SALE_ID",
      "SOLD_ON"
    FROM "menura_11_counted"
    WHERE "menura.related" >= 1 AND "menura.related" = "menura.visible"
  ),
  "menura_13_aggregate" AS (
    SELECT
      CAST(COUNT("SALE_ID") AS BIGINT) AS "metric_1_component_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_12_semijoin"
  ),
  "menura_14_project" AS (
    SELECT
      "metric_1_component_1" AS "sale_count",
      "metric_2_component_1" AS "total_amount"
    FROM "menura_13_aggregate"
  )
SELECT
  "sale_count",
  "total_amount"
FROM "menura_14_project"