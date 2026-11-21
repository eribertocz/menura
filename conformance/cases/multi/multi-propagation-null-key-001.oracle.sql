SELECT customers.name AS name, SUM(orders.amount) AS order_total
FROM shop.main.orders AS orders
JOIN shop.main.customers AS customers ON orders.customer_id = customers.customer_id
JOIN shop.main.stores AS stores ON customers.store_id = stores.store_id
WHERE TRUE
GROUP BY customers.name
