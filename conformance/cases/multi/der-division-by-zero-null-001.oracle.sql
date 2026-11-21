SELECT regions.name AS name,
  CASE WHEN SUM(invoices.billed) = 0 THEN NULL
    ELSE CAST(SUM(invoices.collected) AS DOUBLE) / CAST(SUM(invoices.billed) AS DOUBLE) END AS collection_rate
FROM shop.main.invoices AS invoices
LEFT JOIN shop.main.regions AS regions ON invoices.region_id = regions.region_id
GROUP BY regions.name
