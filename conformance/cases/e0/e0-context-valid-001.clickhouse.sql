WITH
  "menura_1_scan" AS (
    SELECT
      "billed" AS "BILLED",
      "customer_id" AS "CUSTOMER_ID",
      ROW_NUMBER() OVER () AS "menura.row_identity"
    FROM "shop"."invoices"
  ),
  "menura_2_scan" AS (
    SELECT
      "customer_id" AS "INVOICES_CUSTOMERS.CUSTOMER_ID",
      "region" AS "INVOICES_CUSTOMERS.REGION"
    FROM "shop"."customers"
  ),
  "menura_3_join" AS (
    SELECT
      "BILLED",
      "CUSTOMER_ID",
      "menura.row_identity",
      "INVOICES_CUSTOMERS.CUSTOMER_ID",
      "INVOICES_CUSTOMERS.REGION"
    FROM "menura_1_scan"
    LEFT JOIN "menura_2_scan" ON "menura_1_scan"."CUSTOMER_ID" = "menura_2_scan"."INVOICES_CUSTOMERS.CUSTOMER_ID"
  ),
  "menura_4_deduplicate" AS (
    SELECT
      "BILLED",
      "CUSTOMER_ID",
      "menura.row_identity",
      "INVOICES_CUSTOMERS.CUSTOMER_ID",
      "INVOICES_CUSTOMERS.REGION"
    FROM "menura_3_join"
    QUALIFY ROW_NUMBER() OVER (PARTITION BY "INVOICES_CUSTOMERS.REGION", "menura.row_identity") = 1
  ),
  "menura_5_aggregate" AS (
    SELECT
      "INVOICES_CUSTOMERS.REGION" AS "dimension_1",
      accurateCast(sumOrNull(toInt256("BILLED")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_4_deduplicate"
    GROUP BY 1
  ),
  "menura_6_project" AS (
    SELECT
      "dimension_1" AS "region",
      "metric_1_component_1" AS "billed"
    FROM "menura_5_aggregate"
  )
SELECT
  "region",
  "billed"
FROM "menura_6_project"
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