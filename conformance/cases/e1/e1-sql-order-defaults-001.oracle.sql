SELECT
  channel AS channel,
  COUNT(sale_id) AS n
FROM shop.sales
GROUP BY channel
ORDER BY channel ASC NULLS LAST
