WITH invoices_branch AS (
  SELECT regions.name AS name,
    CASE WHEN SUM(invoices.billed) = 0 THEN NULL
      ELSE CAST(SUM(invoices.collected) AS DOUBLE) / CAST(SUM(invoices.billed) AS DOUBLE) END AS collection_rate
  FROM shop.main.invoices AS invoices
  LEFT JOIN shop.main.regions AS regions ON invoices.region_id = regions.region_id
  GROUP BY regions.name
), budgets_branch AS (
  SELECT regions.name AS name, SUM(budgets.amount) AS budget_total
  FROM shop.main.budgets AS budgets
  LEFT JOIN shop.main.regions AS regions ON budgets.region_id = regions.region_id
  GROUP BY regions.name
), stitch_keys AS (
  SELECT name FROM invoices_branch UNION SELECT name FROM budgets_branch
)
SELECT stitch_keys.name,
  CASE WHEN budgets_branch.budget_total = 0 THEN error('RDER-01: division by zero in metric STRICT_RATE_PER_BUDGET')
    ELSE invoices_branch.collection_rate / CAST(budgets_branch.budget_total AS DOUBLE) END AS strict_rate_per_budget
FROM stitch_keys
LEFT JOIN invoices_branch ON stitch_keys.name IS NOT DISTINCT FROM invoices_branch.name
LEFT JOIN budgets_branch ON stitch_keys.name IS NOT DISTINCT FROM budgets_branch.name
