WITH
  "menura_1_periods" AS (
    SELECT
      CAST(CAST(CAST("menura.start" AS DATE) AS TIMESTAMP) AT TIME ZONE CAST($1 AS VARCHAR) AS TIMESTAMP WITH TIME ZONE) AS "PERIOD"
    FROM generate_series(CAST(date_trunc('week', CAST($2 AS DATE)) AS TIMESTAMP), CAST(CAST($3 AS DATE) AS TIMESTAMP), INTERVAL 7 DAY) AS "menura_periods"("menura.start")
  )
SELECT
  "PERIOD"
FROM "menura_1_periods"