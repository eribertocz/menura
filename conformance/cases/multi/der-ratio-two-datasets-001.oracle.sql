WITH invoices_branch AS (
  SELECT regions.name AS name, SUM(invoices.collected) AS collected
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
  CASE WHEN budgets_branch.budget_total = 0 THEN NULL
    ELSE CAST(invoices_branch.collected AS DOUBLE) / CAST(budgets_branch.budget_total AS DOUBLE) END AS collected_to_budget
FROM stitch_keys
LEFT JOIN invoices_branch ON stitch_keys.name IS NOT DISTINCT FROM invoices_branch.name
LEFT JOIN budgets_branch ON stitch_keys.name IS NOT DISTINCT FROM budgets_branch.name
