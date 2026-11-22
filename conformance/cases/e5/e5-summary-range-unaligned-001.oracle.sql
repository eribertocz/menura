SELECT region, SUM(amount) AS total_amount
FROM shop.main.orders
WHERE order_date BETWEEN DATE '2026-10-05' AND DATE '2026-11-30'
GROUP BY 1
