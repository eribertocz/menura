SELECT stores.region AS region, COUNT(customers.customer_id) AS customer_count
FROM shop.main.customers AS customers
JOIN shop.main.stores AS stores ON customers.store_id = stores.store_id
WHERE stores.region IN ('SOUTH')
GROUP BY stores.region
