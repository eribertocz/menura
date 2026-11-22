SELECT order_date, SUM(amount) AS total_amount
FROM shop.main.orders
GROUP BY 1
