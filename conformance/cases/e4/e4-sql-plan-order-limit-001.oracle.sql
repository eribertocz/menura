SELECT
  COUNT(sale_id) AS n,
  region AS region
FROM shop.sales
GROUP BY region
ORDER BY region DESC NULLS LAST
LIMIT 2
