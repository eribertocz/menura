WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "paid" AS "PAID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."main"."balances"
  ),
  "menura_2_scan" AS (
    SELECT
      "category" AS "BALANCES_CUSTOMERS.CATEGORY",
      "customer_id" AS "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."main"."customers"
  ),
  "menura_3_join" AS (
    SELECT
      "CUSTOMER_ID",
      "PAID",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "PAID",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "BALANCES_CUSTOMERS.CATEGORY" AS "dimension_1",
      CAST(SUM("PAID") AS HUGEINT) AS "metric_1_component_1",
      CAST(COUNT("PAID") AS BIGINT) AS "metric_3_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."main"."balances"
  ),
  "menura_7_scan" AS (
    SELECT
      "category" AS "BALANCES_CUSTOMERS.CATEGORY",
      "customer_id" AS "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."main"."customers"
  ),
  "menura_8_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_6_scan"
    LEFT JOIN "menura_7_scan" ON "menura_6_scan"."CUSTOMER_ID" = "menura_7_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_9_deduplicate" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_8_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_10_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "valid_from" AS "VALID_FROM",
      "valid_to" AS "VALID_TO"
    FROM "shop"."main"."members"
  ),
  "menura_11_scan" AS (
    SELECT
      "balance" AS "BALANCE",
      "customer_id" AS "CUSTOMER_ID",
      "period" AS "PERIOD",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."main"."balances"
  ),
  "menura_12_scan" AS (
    SELECT
      "category" AS "BALANCES_CUSTOMERS.CATEGORY",
      "customer_id" AS "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "shop"."main"."customers"
  ),
  "menura_13_join" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_11_scan"
    LEFT JOIN "menura_12_scan" ON "menura_11_scan"."CUSTOMER_ID" = "menura_12_scan"."BALANCES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_14_deduplicate" AS (
    SELECT
      "BALANCE",
      "CUSTOMER_ID",
      "PERIOD",
      "menura.row_identity",
      "BALANCES_CUSTOMERS.CATEGORY",
      "BALANCES_CUSTOMERS.CUSTOMER_ID"
    FROM "menura_13_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "BALANCES_CUSTOMERS.CATEGORY", "menura.row_identity") = 1
  ),
  "menura_15_aggregate" AS (
    SELECT
      CAST(date_trunc('month', "PERIOD") AS DATE) AS "period"
    FROM "menura_14_deduplicate"
    GROUP BY 1
  ),
  "menura_16_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."period" AS "period"
    FROM "menura_10_scan" AS "menura_row"
    JOIN "menura_15_aggregate" AS "menura_period" ON "menura_row"."VALID_FROM" <= CAST("menura_period"."period" + INTERVAL 1 MONTH - INTERVAL 1 DAY AS DATE) AND ("menura_row"."VALID_TO" IS NULL OR "menura_row"."VALID_TO" >= "menura_period"."period")
    WHERE "menura_row"."CUSTOMER_ID" IS NOT NULL
    GROUP BY 1, 2
  ),
  "menura_17_values" AS (
    SELECT
      "BALANCES_CUSTOMERS.CATEGORY" AS "dimension_1",
      "CUSTOMER_ID" AS "entity_1",
      CAST(date_trunc('month', "PERIOD") AS DATE) AS "menura.period",
      SUM("BALANCE") AS "metric_2_component_1"
    FROM "menura_9_deduplicate"
    GROUP BY 1, 2, 3
  ),
  "menura_18_checked" AS (
    SELECT
      "menura_value"."dimension_1",
      "menura_value"."entity_1",
      "menura_value"."menura.period",
      "menura_value"."metric_2_component_1" AS "metric_2_component_1"
    FROM "menura_17_values" AS "menura_value"
    LEFT JOIN "menura_16_population" AS "menura_expected" ON "menura_value"."entity_1" = "menura_expected"."CUSTOMER_ID" AND "menura_value"."menura.period" = "menura_expected"."period"
    WHERE "menura_expected"."period" IS NOT NULL
  ),
  "menura_19_selected" AS (
    SELECT
      "dimension_1",
      MAX("menura.period") AS "menura.period"
    FROM "menura_17_values"
    GROUP BY 1
  ),
  "menura_20_snapshot" AS (
    SELECT
      "menura_candidate"."dimension_1",
      "menura_candidate"."entity_1",
      CAST("menura_candidate"."metric_2_component_1" AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_18_checked" AS "menura_candidate"
    JOIN "menura_19_selected" AS "menura_selected" ON "menura_candidate"."dimension_1" IS NOT DISTINCT FROM "menura_selected"."dimension_1" AND "menura_candidate"."menura.period" = "menura_selected"."menura.period"
  ),
  "menura_21_aggregate" AS (
    SELECT
      "dimension_1" AS "dimension_1",
      CAST(SUM("metric_2_component_1") AS HUGEINT) AS "metric_2_component_1"
    FROM "menura_20_snapshot"
    GROUP BY 1
  ),
  "menura_22_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      "metric_3_component_1",
      TRUE AS "menura.present"
    FROM "menura_5_aggregate"
  ),
  "menura_23_present" AS (
    SELECT
      "dimension_1",
      "metric_2_component_1",
      TRUE AS "menura.present"
    FROM "menura_21_aggregate"
  ),
  "menura_24_keys" AS (
    SELECT
      "dimension_1"
    FROM "menura_5_aggregate"
    UNION
    SELECT
      "dimension_1"
    FROM "menura_21_aggregate"
  ),
  "menura_25_stitch" AS (
    SELECT
      "menura_keys"."dimension_1" AS "dimension_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_1_component_1" ELSE NULL END AS "metric_1_component_1",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."metric_3_component_1" ELSE CAST(0 AS BIGINT) END AS "metric_3_component_1",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."metric_2_component_1" ELSE NULL END AS "metric_2_component_1"
    FROM "menura_24_keys" AS "menura_keys"
    LEFT JOIN "menura_22_present" AS "menura_branch_1" ON "menura_keys"."dimension_1" IS NOT DISTINCT FROM "menura_branch_1"."dimension_1"
    LEFT JOIN "menura_23_present" AS "menura_branch_2" ON "menura_keys"."dimension_1" IS NOT DISTINCT FROM "menura_branch_2"."dimension_1"
  ),
  "menura_26_project" AS (
    SELECT
      "dimension_1" AS "category",
      "metric_1_component_1" AS "total_paid",
      "metric_2_component_1" AS "last_global",
      "metric_3_component_1" AS "paid_rows"
    FROM "menura_25_stitch"
  )
SELECT
  "category",
  "total_paid",
  "last_global",
  "paid_rows"
FROM "menura_26_project"
