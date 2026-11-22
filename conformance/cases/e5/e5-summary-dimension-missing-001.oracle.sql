SELECT amount, SUM(amount) AS total_amount
FROM shop.main.orders
GROUP BY 1
