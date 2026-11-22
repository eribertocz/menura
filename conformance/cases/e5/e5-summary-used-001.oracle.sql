SELECT region, CAST(date_trunc('month', order_date) AS DATE) AS order_date, SUM(amount) AS total_amount, COUNT(order_id) AS order_count, AVG(amount) AS average_amount
FROM shop.main.orders
GROUP BY 1, 2
