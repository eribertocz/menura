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
  "menura_3_rows" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SOLD_ON",
      ROW_NUMBER() OVER () AS "menura.row"
    FROM "menura_1_scan"
  ),
  "menura_4_versions" AS (
    SELECT
      "menura_row"."AMOUNT" AS "AMOUNT",
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_row"."SOLD_ON" AS "SOLD_ON",
      "menura_row"."menura.row" AS "menura.row",
      "menura_version"."SALE_CUSTOMER.CUSTOMER_ID" AS "SALE_CUSTOMER.CUSTOMER_ID",
      "menura_version"."SALE_CUSTOMER.SEGMENT" AS "SALE_CUSTOMER.SEGMENT",
      "menura_version"."SALE_CUSTOMER.VALID_FROM" AS "SALE_CUSTOMER.VALID_FROM",
      "menura_version"."SALE_CUSTOMER.VALID_TO" AS "SALE_CUSTOMER.VALID_TO",
      CASE WHEN "menura_version"."SALE_CUSTOMER.VALID_FROM" IS NOT NULL AND "menura_version"."SALE_CUSTOMER.VALID_FROM" <= "menura_row"."SOLD_ON" AND (nullIf("menura_version"."SALE_CUSTOMER.VALID_TO", toDate32('2099-12-31')) IS NULL OR "menura_row"."SOLD_ON" < nullIf("menura_version"."SALE_CUSTOMER.VALID_TO", toDate32('2099-12-31'))) THEN 1 ELSE 0 END AS "menura.covers"
    FROM "menura_3_rows" AS "menura_row"
    LEFT JOIN "menura_2_scan" AS "menura_version" ON "menura_row"."CUSTOMER_ID" = "menura_version"."SALE_CUSTOMER.CUSTOMER_ID"
  ),
  "menura_5_covered" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SOLD_ON",
      "SALE_CUSTOMER.CUSTOMER_ID",
      "SALE_CUSTOMER.SEGMENT",
      "SALE_CUSTOMER.VALID_FROM",
      "SALE_CUSTOMER.VALID_TO",
      "menura.covers",
      MAX("menura.covers") OVER (PARTITION BY "menura.row") AS "menura.covered",
      ROW_NUMBER() OVER (PARTITION BY "menura.row") AS "menura.rank"
    FROM "menura_4_versions"
  ),
  "menura_6_join" AS (
    SELECT
      "AMOUNT",
      "CUSTOMER_ID",
      "SOLD_ON",
      "SALE_CUSTOMER.CUSTOMER_ID",
      "SALE_CUSTOMER.SEGMENT",
      "SALE_CUSTOMER.VALID_FROM",
      "SALE_CUSTOMER.VALID_TO"
    FROM "menura_5_covered"
    WHERE "menura.covers" = 1
  ),
  "menura_7_aggregate" AS (
    SELECT
      "SALE_CUSTOMER.SEGMENT" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_6_join"
    GROUP BY 1
  ),
  "menura_8_project" AS (
    SELECT
      "dimension_1" AS "segment",
      "metric_1_component_1" AS "total_amount"
    FROM "menura_7_aggregate"
  )
SELECT
  "segment",
  "total_amount"
FROM "menura_8_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1,
  use_variant_as_common_type = 0