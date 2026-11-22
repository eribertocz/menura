SELECT
  CASE WHEN channel = 'web' THEN 'online' ELSE 'offline' END AS kind,
  SUM(amount) AS amount
FROM shop.main.sales
WHERE quantity > 0
GROUP BY 1
ORDER BY kind ASC NULLS LAST
