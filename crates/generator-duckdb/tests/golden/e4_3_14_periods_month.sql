WITH
  "menura_1_periods" AS (
    SELECT
      CAST("menura.start" AS DATE) AS "PERIOD"
    FROM generate_series(CAST(date_trunc('month', CAST($1 AS DATE)) AS TIMESTAMP), CAST(CAST($2 AS DATE) AS TIMESTAMP), INTERVAL 1 MONTH) AS "menura_periods"("menura.start")
  )
SELECT
  "PERIOD"
FROM "menura_1_periods"