SELECT
  CAST(SUM(amount) AS DOUBLE) / SUM(quantity) AS per_unit,
  region AS region,
  COALESCE(COUNT(sale_id), 0) * 2 AS doubled
FROM shop.main.sales
GROUP BY region
ORDER BY region ASC NULLS LAST
