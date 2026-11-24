SELECT CAST(date_trunc('month', order_date) AS DATE) AS order_date, COUNT(order_id) AS order_count
FROM shop.orders
GROUP BY 1
