SELECT SUM(lines.quantity) AS line_quantity
FROM shop.lines AS lines
JOIN shop.products AS products ON lines.product_id = products.product_id
JOIN shop.suppliers AS product_suppliers ON products.supplier_id = product_suppliers.supplier_id
JOIN shop.warehouses AS warehouses ON lines.warehouse_id = warehouses.warehouse_id
JOIN shop.suppliers AS warehouse_suppliers ON warehouses.supplier_id = warehouse_suppliers.supplier_id
WHERE product_suppliers.region IN ('NORTH')
  AND warehouse_suppliers.region IN ('NORTH')
