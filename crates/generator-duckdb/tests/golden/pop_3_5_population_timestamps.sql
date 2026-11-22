WITH
  "menura_1_scan" AS (
    SELECT
      "customer_id" AS "CUSTOMER_ID",
      "active_from" AS "ACTIVE_FROM",
      "active_to" AS "ACTIVE_TO"
    FROM "main"."customers"
  ),
  "menura_2_scan" AS (
    SELECT
      "period" AS "PERIOD"
    FROM "main"."months"
  ),
  "menura_3_population" AS (
    SELECT
      "menura_row"."CUSTOMER_ID" AS "CUSTOMER_ID",
      "menura_period"."PERIOD" AS "PERIOD"
    FROM "menura_1_scan" AS "menura_row"
    JOIN "menura_2_scan" AS "menura_period" ON CAST(("menura_row"."ACTIVE_FROM") AT TIME ZONE CAST($1 AS VARCHAR) AS DATE) <= CAST("menura_period"."PERIOD" + INTERVAL 7 DAY - INTERVAL 1 DAY AS DATE) AND (CAST("menura_row"."ACTIVE_TO" AS DATE) IS NULL OR CAST("menura_row"."ACTIVE_TO" AS DATE) >= "menura_period"."PERIOD")
    GROUP BY 1, 2
  )
SELECT
  "CUSTOMER_ID",
  "PERIOD"
FROM "menura_3_population"