WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "quantity" AS "QUANTITY",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."orders"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "name" AS "ORDERS_CUSTOMERS.NAME"
    FROM "shop"."customers"
  ),
  "menura_3_join" AS (
    SELECT
      "CUSTOMER_ID",
      "QUANTITY",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.NAME"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON CAST("menura_1_scan"."CUSTOMER_ID" AS Nullable(Int128)) = CAST("menura_2_scan"."ORDERS_CUSTOMERS.CUSTOMER_ID" AS Nullable(Int128))
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "CUSTOMER_ID",
      "QUANTITY",
      "menura.row_identity",
      "ORDERS_CUSTOMERS.CUSTOMER_ID",
      "ORDERS_CUSTOMERS.NAME"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "ORDERS_CUSTOMERS.NAME", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "ORDERS_CUSTOMERS.NAME" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "name",
      "metric_1_component_1" AS "total_quantity"
    FROM "menura_5_aggregate"
  )
SELECT
  "name",
  "total_quantity"
FROM "menura_6_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1