WITH
  "menura_1_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_2_filter" AS (
    SELECT
      "AMOUNT",
      "ORDER_DATE"
    FROM "menura_1_scan"
    WHERE ("ORDER_DATE" >= {p1:Date32}) AND ("ORDER_DATE" <= {p2:Date32})
  ),
  "menura_3_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("ORDER_DATE")) AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_2_component_1"
    FROM "menura_2_filter"
    GROUP BY 1
  ),
  "menura_4_scan" AS (
    SELECT
      "amount" AS "AMOUNT",
      "order_date" AS "ORDER_DATE"
    FROM "shop"."orders"
  ),
  "menura_5_todate" AS (
    SELECT
      "menura_rows"."AMOUNT" AS "AMOUNT",
      "menura_rows"."ORDER_DATE" AS "ORDER_DATE",
      "menura_start" AS "menura.period"
    FROM "menura_4_scan" AS "menura_rows"
    ARRAY JOIN arrayMap(menura_offset -> addMonths(assumeNotNull(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE"))), menura_offset), range(toUInt64(if(ifNull(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE"))), addYears(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE")), 1)) < 0, 1), 0, intDiv(dateDiff('month', assumeNotNull(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE"))), addYears(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE")), 1)), 1) + 1)))) AS "menura_start"
    WHERE "menura_rows"."ORDER_DATE" IS NOT NULL AND toDate32(toStartOfYear("menura_start")) = toDate32(toStartOfYear(toDate32(toStartOfMonth("menura_rows"."ORDER_DATE")))) AND "menura_start" <= {p4:Date32} AND addMonths("menura_start", 1) > {p3:Date32}
  ),
  "menura_6_aggregate" AS (
    SELECT
      toDate32(toStartOfMonth("menura.period")) AS "dimension_1",
      accurateCast(sumOrNull(toInt256("AMOUNT")), 'Nullable(Int128)') AS "metric_1_component_1"
    FROM "menura_5_todate"
    GROUP BY 1
  ),
  "menura_7_present" AS (
    SELECT
      "dimension_1",
      "metric_1_component_1",
      TRUE AS "menura.present"
    FROM "menura_6_aggregate"
  ),
  "menura_8_lookup" AS (
    SELECT
      "menura_result"."dimension_1" AS "dimension_1",
      "menura_result"."metric_1_component_1" AS "metric_1_component_1",
      "menura_result"."metric_2_component_1" AS "metric_2_component_1",
      CASE WHEN "menura_context"."menura.present" THEN "menura_context"."metric_1_component_1" ELSE NULL END AS "context_1_component_1"
    FROM "menura_3_aggregate" AS "menura_result"
    LEFT JOIN "menura_7_present" AS "menura_context" ON isNotDistinctFrom("menura_result"."dimension_1", "menura_context"."dimension_1")
  ),
  "menura_9_project" AS (
    SELECT
      "dimension_1" AS "order_date",
      "metric_1_component_1" AS "sales",
      "context_1_component_1" AS "sales_ytd"
    FROM "menura_8_lookup"
  )
SELECT
  "order_date",
  "sales",
  "sales_ytd"
FROM "menura_9_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1