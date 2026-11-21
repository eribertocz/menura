SELECT COUNT(visits.visit_id) AS visit_count
FROM shop.main.visits AS visits
WHERE EXISTS (
    SELECT 1 FROM shop.main.zones AS zones WHERE zones.zone_code = visits.zone_code
  )
  AND NOT EXISTS (
    SELECT 1 FROM shop.main.zones AS zones
    WHERE zones.zone_code = visits.zone_code AND NOT (zones.region IN ('NORTH'))
  )
