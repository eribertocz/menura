SELECT DISTINCT customers.name AS name
FROM shop.customers AS customers
JOIN shop.stores AS stores ON customers.store_id = stores.store_id
WHERE stores.region IN ('NORTH')
