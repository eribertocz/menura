SELECT
  region AS region,
  SUM(amount) AS total_amount,
  SUM(quantity) AS total_quantity,
  COUNT(sale_id) AS sale_count
FROM shop.main.sales
WHERE NOT voided
GROUP BY region
