SELECT SUM(lines.quantity) AS line_quantity
FROM shop.main.lines AS lines
JOIN shop.main.products AS products ON lines.product_id = products.product_id
JOIN shop.main.suppliers AS product_suppliers ON products.supplier_id = product_suppliers.supplier_id
JOIN shop.main.warehouses AS warehouses ON lines.warehouse_id = warehouses.warehouse_id
JOIN shop.main.suppliers AS warehouse_suppliers ON warehouses.supplier_id = warehouse_suppliers.supplier_id
WHERE product_suppliers.region IN ('SOUTH')
  AND warehouse_suppliers.region IN ('SOUTH')
