SELECT region, SUM(amount) AS total_amount
FROM shop.main.orders
WHERE region = 'SOUTH'
GROUP BY 1
