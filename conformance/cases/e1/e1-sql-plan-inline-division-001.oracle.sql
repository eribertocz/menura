SELECT
  channel AS channel,
  CASE WHEN SUM(quantity) = 0 THEN NULL ELSE SUM(amount) / SUM(quantity) END AS per_unit
FROM shop.main.sales
WHERE quantity = 0
GROUP BY channel
