WITH
  "menura_1_scan" AS (
    SELECT
      "Sales Region" AS "REGION"
    FROM (select * from "raw"."orders")
  )
SELECT
  "REGION"
FROM "menura_1_scan"