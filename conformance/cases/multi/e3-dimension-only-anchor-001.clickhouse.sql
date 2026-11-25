WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "status" AS "STATUS"
    FROM "shop"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "customer_name" AS "ORDERS_CUSTOMERS.CUSTOMER_NAME"
    FROM "shop"."customers"
  ),
  "menura_3_join" AS (
    SELECT
      "CUSTOMER_ID",
      "STATUS",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.CUSTOMER_NAME"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_4_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.CUSTOMER_NAME" AS "dimension_1",
      "STATUS" AS "dimension_2"
    FROM "menura_3_join"
    GROUP BY 1, 2
  ),
  "menura_5_project" AS (
    SELECT
      "dimension_1" AS "customer_name",
      "dimension_2" AS "status"
    FROM "menura_4_aggregate"
  )
SELECT
  "customer_name",
  "status"
FROM "menura_5_project"
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