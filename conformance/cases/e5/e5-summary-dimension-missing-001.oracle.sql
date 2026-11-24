SELECT amount, SUM(amount) AS total_amount
FROM shop.orders
GROUP BY 1
