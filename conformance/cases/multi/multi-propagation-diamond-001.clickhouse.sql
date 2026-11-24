WITH
  "menura_1_scan" AS (
    SELECT
      "product_id" AS "PRODUCT_ID",
      "quantity" AS "QUANTITY",
      "warehouse_id" AS "WAREHOUSE_ID"
    FROM "shop"."lines"
  ),
  "menura_2_scan" AS (
    SELECT
      "product_id" AS "PRODUCT_ID"
    FROM "shop"."products"
  ),
  "menura_3_scan" AS (
    SELECT
      "product_id" AS "PRODUCT_ID",
      "supplier_id" AS "SUPPLIER_ID"
    FROM "shop"."products"
  ),
  "menura_4_scan" AS (
    SELECT
      "supplier_id" AS "SUPPLIER_ID"
    FROM "shop"."suppliers"
  ),
  "menura_5_scan" AS (
    SELECT
      "region" AS "REGION",
      "supplier_id" AS "SUPPLIER_ID"
    FROM "shop"."suppliers"
  ),
  "menura_6_filter" AS (
    SELECT
      "REGION",
      "SUPPLIER_ID"
    FROM "menura_5_scan"
    WHERE (CASE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END) AND TRUE
  ),
  "menura_7_related" AS (
    SELECT
      "SUPPLIER_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_4_scan"
    GROUP BY 1
  ),
  "menura_8_visible" AS (
    SELECT
      "SUPPLIER_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_6_filter"
    GROUP BY 1
  ),
  "menura_9_semijoin" AS (
    SELECT
      "menura_row"."PRODUCT_ID",
      "menura_row"."SUPPLIER_ID"
    FROM "menura_3_scan" AS "menura_row"
    JOIN "menura_7_related" AS "menura_related" ON "menura_row"."SUPPLIER_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_8_visible" AS "menura_visible" ON "menura_row"."SUPPLIER_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_10_related" AS (
    SELECT
      "PRODUCT_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_11_visible" AS (
    SELECT
      "PRODUCT_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_9_semijoin"
    GROUP BY 1
  ),
  "menura_12_semijoin" AS (
    SELECT
      "menura_row"."PRODUCT_ID",
      "menura_row"."QUANTITY",
      "menura_row"."WAREHOUSE_ID"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_10_related" AS "menura_related" ON "menura_row"."PRODUCT_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_11_visible" AS "menura_visible" ON "menura_row"."PRODUCT_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_13_scan" AS (
    SELECT
      "warehouse_id" AS "WAREHOUSE_ID"
    FROM "shop"."warehouses"
  ),
  "menura_14_scan" AS (
    SELECT
      "supplier_id" AS "SUPPLIER_ID",
      "warehouse_id" AS "WAREHOUSE_ID"
    FROM "shop"."warehouses"
  ),
  "menura_15_scan" AS (
    SELECT
      "supplier_id" AS "SUPPLIER_ID"
    FROM "shop"."suppliers"
  ),
  "menura_16_scan" AS (
    SELECT
      "region" AS "REGION",
      "supplier_id" AS "SUPPLIER_ID"
    FROM "shop"."suppliers"
  ),
  "menura_17_filter" AS (
    SELECT
      "REGION",
      "SUPPLIER_ID"
    FROM "menura_16_scan"
    WHERE (CASE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END) AND TRUE
  ),
  "menura_18_related" AS (
    SELECT
      "SUPPLIER_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_15_scan"
    GROUP BY 1
  ),
  "menura_19_visible" AS (
    SELECT
      "SUPPLIER_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_17_filter"
    GROUP BY 1
  ),
  "menura_20_semijoin" AS (
    SELECT
      "menura_row"."SUPPLIER_ID",
      "menura_row"."WAREHOUSE_ID"
    FROM "menura_14_scan" AS "menura_row"
    JOIN "menura_18_related" AS "menura_related" ON "menura_row"."SUPPLIER_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_19_visible" AS "menura_visible" ON "menura_row"."SUPPLIER_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_21_related" AS (
    SELECT
      "WAREHOUSE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_13_scan"
    GROUP BY 1
  ),
  "menura_22_visible" AS (
    SELECT
      "WAREHOUSE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_20_semijoin"
    GROUP BY 1
  ),
  "menura_23_semijoin" AS (
    SELECT
      "menura_row"."PRODUCT_ID",
      "menura_row"."QUANTITY",
      "menura_row"."WAREHOUSE_ID"
    FROM "menura_12_semijoin" AS "menura_row"
    JOIN "menura_21_related" AS "menura_related" ON "menura_row"."WAREHOUSE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_22_visible" AS "menura_visible" ON "menura_row"."WAREHOUSE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_24_aggregate" AS (
    SELECT
      accurateCast(sumOrNull(toInt256("QUANTITY")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_23_semijoin"
  ),
  "menura_25_project" AS (
    SELECT
      "metric_1_component_1" AS "line_quantity"
    FROM "menura_24_aggregate"
  )
SELECT
  "line_quantity"
FROM "menura_25_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1