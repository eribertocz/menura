SELECT SUM(orders.amount) AS order_total
FROM shop.orders AS orders
JOIN shop.customers AS customers ON orders.customer_id = customers.customer_id
JOIN shop.stores AS stores ON customers.store_id = stores.store_id
WHERE TRUE
  AND stores.region IN ('SOUTH')
