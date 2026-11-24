SELECT COUNT(visits.visit_id) AS visit_count
FROM shop.visits AS visits
WHERE EXISTS (
    SELECT 1 FROM shop.zones AS zones WHERE zones.zone_code = visits.zone_code
  )
  AND NOT EXISTS (
    SELECT 1 FROM shop.zones AS zones
    WHERE zones.zone_code = visits.zone_code AND NOT (TRUE)
  )
