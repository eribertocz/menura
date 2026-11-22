SELECT
  COUNT(sale_id) AS n,
  region AS region
FROM shop.main.sales
GROUP BY region
ORDER BY region DESC NULLS LAST
LIMIT 2
