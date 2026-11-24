WITH
  "menura_1_scan" AS (
    SELECT
      "desk_id" AS "DESK_ID",
      "queue_id" AS "QUEUE_ID",
      "ticket_id" AS "TICKET_ID"
    FROM "shop"."tickets"
  ),
  "menura_2_scan" AS (
    SELECT
      "desk_id" AS "DESK_ID"
    FROM "shop"."desks"
  ),
  "menura_3_scan" AS (
    SELECT
      "desk_id" AS "DESK_ID",
      "region" AS "REGION"
    FROM "shop"."desks"
  ),
  "menura_4_filter" AS (
    SELECT
      "DESK_ID",
      "REGION"
    FROM "menura_3_scan"
    WHERE TRUE AND (CASE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END)
  ),
  "menura_5_related" AS (
    SELECT
      "DESK_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_2_scan"
    GROUP BY 1
  ),
  "menura_6_visible" AS (
    SELECT
      "DESK_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_4_filter"
    GROUP BY 1
  ),
  "menura_7_semijoin" AS (
    SELECT
      "menura_row"."DESK_ID",
      "menura_row"."QUEUE_ID",
      "menura_row"."TICKET_ID"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_5_related" AS "menura_related" ON "menura_row"."DESK_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_6_visible" AS "menura_visible" ON "menura_row"."DESK_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_8_scan" AS (
    SELECT
      "queue_id" AS "QUEUE_ID"
    FROM "shop"."queues"
  ),
  "menura_9_scan" AS (
    SELECT
      "queue_id" AS "QUEUE_ID",
      "region" AS "REGION"
    FROM "shop"."queues"
  ),
  "menura_10_filter" AS (
    SELECT
      "QUEUE_ID",
      "REGION"
    FROM "menura_9_scan"
    WHERE TRUE AND (CASE WHEN "REGION" IS NULL THEN NULL WHEN has({p1:Array(String)}, assumeNotNull("REGION")) THEN TRUE WHEN has({p1:Array(String)}, NULL) THEN NULL ELSE FALSE END)
  ),
  "menura_11_related" AS (
    SELECT
      "QUEUE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_8_scan"
    GROUP BY 1
  ),
  "menura_12_visible" AS (
    SELECT
      "QUEUE_ID" AS "menura.key_1",
      coalesce(count(), 0) AS "menura.rows"
    FROM "menura_10_filter"
    GROUP BY 1
  ),
  "menura_13_semijoin" AS (
    SELECT
      "menura_row"."DESK_ID",
      "menura_row"."QUEUE_ID",
      "menura_row"."TICKET_ID"
    FROM "menura_7_semijoin" AS "menura_row"
    JOIN "menura_11_related" AS "menura_related" ON "menura_row"."QUEUE_ID" = "menura_related"."menura.key_1"
    LEFT JOIN "menura_12_visible" AS "menura_visible" ON "menura_row"."QUEUE_ID" = "menura_visible"."menura.key_1"
    WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows"
  ),
  "menura_14_aggregate" AS (
    SELECT
      accurateCast(coalesce(count("TICKET_ID"), 0), 'Nullable(Int64)') AS "metric_1_component_1"
    FROM "menura_13_semijoin"
  ),
  "menura_15_project" AS (
    SELECT
      "metric_1_component_1" AS "ticket_count"
    FROM "menura_14_aggregate"
  )
SELECT
  "ticket_count"
FROM "menura_15_project"
SETTINGS
  join_use_nulls = 1,
  transform_null_in = 0,
  prefer_column_name_to_alias = 1,
  enable_positional_arguments = 1,
  short_circuit_function_evaluation = 'force_enable',
  enable_extended_results_for_datetime_functions = 1,
  aggregate_functions_null_for_empty = 1,
  decimal_check_overflow = 1