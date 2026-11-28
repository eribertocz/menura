WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "customer_id" AS "CUSTOMER_ID",
      "sold_on" AS "SOLD_ON"
    FROM "shop"."sales"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "SALE_CUSTOMER.CUSTOMER_ID",
      "segment" AS "SALE_CUSTOMER.SEGMENT",
      "valid_from" AS "SALE_CUSTOMER.VALID_FROM",
      "valid_to" AS "SALE_CUSTOMER.VALID_TO"
    FROM "shop"."customer_versions"
  ),
  "menura_3_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SOLD_ON",
      "SALE_CUSTOMER.CUSTOMER_ID",
      "SALE_CUSTOMER.SEGMENT",
      "SALE_CUSTOMER.VALID_FROM",
      "SALE_CUSTOMER.VALID_TO"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON ("menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."SALE_CUSTOMER.CUSTOMER_ID") AND ("menura_2_scan"."SALE_CUSTOMER.VALID_FROM" IS NOT NULL) AND ("menura_2_scan"."SALE_CUSTOMER.VALID_FROM" <= "menura_1_scan"."SOLD_ON") AND ((NULLIF("menura_2_scan"."SALE_CUSTOMER.VALID_TO", CAST('2099-12-31' AS DATE)) IS NULL OR "menura_1_scan"."SOLD_ON" < NULLIF("menura_2_scan"."SALE_CUSTOMER.VALID_TO", CAST('2099-12-31' AS DATE))))
  ),
  "menura_4_aggregate" AS (
    SELECT
      "SALE_CUSTOMER.SEGMENT" AS "dimension_1",
      CAST(SUM("AMOUNT") AS HUGEINT) AS "metric_1_component_1"
    FROM "menura_3_join"
    GROUP BY 1
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "segment",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_4_aggregate"
  )
SELECT
  "segment",
  "total_amount"
FROM "menura_5_project"