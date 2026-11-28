WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "sale_id" AS "SALE_ID",
      "sold_on" AS "SOLD_ON",
      ROW_NUMBER() OVER () AS "menura.row_identity"
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
      "CUSTOMER_ID",
      "SALE_ID",
      "SOLD_ON",
      "menura.row_identity",
      "SALE_CUSTOMER.CUSTOMER_ID",
      "SALE_CUSTOMER.SEGMENT",
      "SALE_CUSTOMER.VALID_FROM",
      "SALE_CUSTOMER.VALID_TO"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON ("menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."SALE_CUSTOMER.CUSTOMER_ID") AND ("menura_2_scan"."SALE_CUSTOMER.VALID_FROM" IS NOT NULL) AND ("menura_2_scan"."SALE_CUSTOMER.VALID_FROM" <= "menura_1_scan"."SOLD_ON") AND ((NULLIF("menura_2_scan"."SALE_CUSTOMER.VALID_TO", CAST('2099-12-31' AS DATE)) IS NULL OR "menura_1_scan"."SOLD_ON" < NULLIF("menura_2_scan"."SALE_CUSTOMER.VALID_TO", CAST('2099-12-31' AS DATE))))
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "SALE_ID",
      "SOLD_ON",
      "menura.row_identity",
      "SALE_CUSTOMER.CUSTOMER_ID",
      "SALE_CUSTOMER.SEGMENT",
      "SALE_CUSTOMER.VALID_FROM",
      "SALE_CUSTOMER.VALID_TO"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "SALE_ID", "SALE_CUSTOMER.SEGMENT", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "SALE_ID" AS "dimension_1",
      "SALE_CUSTOMER.SEGMENT" AS "dimension_2",
      CAST(COUNT("SALE_ID") AS BIGINT) AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1, 2
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "sale_id",
      "dimension_2" AS "segment",
      "metric_1_component_1" AS "sale_count"
    FROM "menura_5_aggregate"
  )
SELECT
  "sale_id",
  "segment",
  "sale_count"
FROM "menura_6_project"