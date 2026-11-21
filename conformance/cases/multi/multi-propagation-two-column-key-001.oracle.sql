SELECT COUNT(items.item_id) AS item_count
FROM shop.main.items AS items
WHERE EXISTS (
    SELECT 1 FROM shop.main.shelves AS shelves
    WHERE shelves.site_code = items.site_code AND shelves.shelf_no = items.shelf_no
  )
  AND NOT EXISTS (
    SELECT 1 FROM shop.main.shelves AS shelves
    WHERE shelves.site_code = items.site_code AND shelves.shelf_no = items.shelf_no
      AND NOT (shelves.region IN ('NORTH'))
  )
