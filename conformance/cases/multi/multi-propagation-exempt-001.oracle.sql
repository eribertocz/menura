SELECT COUNT(customers.customer_id) AS customer_count
FROM shop.customers AS customers
JOIN shop.stores AS stores ON customers.store_id = stores.store_id
