SELECT
  sales.region AS region,
  SUM(returns.refund) AS total_refund
FROM shop.returns AS returns
JOIN shop.sales AS sales ON returns.sale_id = sales.sale_id
GROUP BY sales.region
