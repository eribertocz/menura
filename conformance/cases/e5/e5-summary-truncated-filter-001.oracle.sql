SELECT region, SUM(amount) AS total_amount
FROM shop.main.orders
WHERE order_date >= DATE '2026-10-10'
GROUP BY 1
