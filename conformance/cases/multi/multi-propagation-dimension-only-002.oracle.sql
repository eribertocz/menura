SELECT DISTINCT customers.name AS name
FROM shop.main.customers AS customers
JOIN shop.main.stores AS stores ON customers.store_id = stores.store_id
WHERE stores.region IN ('SOUTH')
