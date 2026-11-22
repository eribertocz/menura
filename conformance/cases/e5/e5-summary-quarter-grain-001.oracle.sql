SELECT CAST(date_trunc('quarter', order_date) AS DATE) AS order_date, SUM(amount) AS total_amount
FROM shop.main.orders
GROUP BY 1
