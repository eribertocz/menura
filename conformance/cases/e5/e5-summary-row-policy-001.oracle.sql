SELECT CAST(date_trunc('month', order_date) AS DATE) AS order_date, SUM(amount) AS total_amount, COUNT(order_id) AS order_count
FROM shop.orders
WHERE region = 'NORTH'
GROUP BY 1
