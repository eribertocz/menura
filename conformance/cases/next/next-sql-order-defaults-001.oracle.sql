SELECT
  channel AS channel,
  COUNT(sale_id) AS n
FROM shop.main.sales
GROUP BY channel
ORDER BY channel ASC NULLS LAST
