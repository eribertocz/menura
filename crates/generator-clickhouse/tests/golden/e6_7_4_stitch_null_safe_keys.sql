WITH
  "menura_1_scan" AS (
    SELECT
      "quantity" AS "QUANTITY",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_2_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      accurateCast(coalesce(count(), 0), 'Nullable(Int64)') AS "ORDER_COUNT",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "TOTAL_QUANTITY"
    FROM "menura_1_scan"
    GROUP BY 1
  ),
  "menura_3_scan" AS (
    SELECT
      "Amount" AS "AMOUNT",
      "Sales Region" AS "REGION"
    FROM "main"."orders"
  ),
  "menura_4_aggregate" AS (
    SELECT
      "REGION" AS "REGION",
      CAST(sumOrNull("AMOUNT") AS Nullable(Decimal(38, 2))) AS "TOTAL_AMOUNT"
    FROM "menura_3_scan"
    GROUP BY 1
  ),
  "menura_5_present" AS (
    SELECT
      "REGION",
      "ORDER_COUNT",
      "TOTAL_QUANTITY",
      TRUE AS "menura.present"
    FROM "menura_2_aggregate"
  ),
  "menura_6_present" AS (
    SELECT
      "REGION",
      "TOTAL_AMOUNT",
      TRUE AS "menura.present"
    FROM "menura_4_aggregate"
  ),
  "menura_7_keys" AS (
    SELECT
      "REGION"
    FROM "menura_2_aggregate"
    UNION DISTINCT
    SELECT
      "REGION"
    FROM "menura_4_aggregate"
  ),
  "menura_8_stitch" AS (
    SELECT
      "menura_keys"."REGION" AS "REGION",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."ORDER_COUNT" ELSE accurateCast(toInt64(0), 'Nullable(Int64)') END AS "ORDER_COUNT",
      CASE WHEN "menura_branch_1"."menura.present" THEN "menura_branch_1"."TOTAL_QUANTITY" ELSE NULL END AS "TOTAL_QUANTITY",
      CASE WHEN "menura_branch_2"."menura.present" THEN "menura_branch_2"."TOTAL_AMOUNT" ELSE NULL END AS "TOTAL_AMOUNT"
    FROM "menura_7_keys" AS "menura_keys"
    LEFT JOIN "menura_5_present" AS "menura_branch_1" ON isNotDistinctFrom("menura_keys"."REGION", "menura_branch_1"."REGION")
    LEFT JOIN "menura_6_present" AS "menura_branch_2" ON isNotDistinctFrom("menura_keys"."REGION", "menura_branch_2"."REGION")
  )
SELECT
  "REGION",
  "ORDER_COUNT",
  "TOTAL_QUANTITY",
  "TOTAL_AMOUNT"
FROM "menura_8_stitch"
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
