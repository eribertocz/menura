SELECT CAST(date_trunc('month', order_date) AS DATE) AS order_date, SUM(price) AS total_price
FROM shop.main.orders
GROUP BY 1
