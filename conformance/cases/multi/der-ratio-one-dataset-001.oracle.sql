SELECT CASE WHEN SUM(billed) = 0 THEN NULL ELSE CAST(SUM(collected) AS DOUBLE) / CAST(SUM(billed) AS DOUBLE) END AS collection_rate
FROM shop.invoices
