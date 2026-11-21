WITH sales_branch AS (
  SELECT regions.name AS name, SUM(sales.amount) AS total_sales
  FROM shop.main.sales AS sales
  LEFT JOIN shop.main.regions AS regions ON sales.region_id = regions.region_id
  GROUP BY regions.name
), budgets_branch AS (
  SELECT regions.name AS name, SUM(budgets.amount) AS budget_total
  FROM shop.main.budgets AS budgets
  LEFT JOIN shop.main.regions AS regions ON budgets.region_id = regions.region_id
  GROUP BY regions.name
), stitch_keys AS (
  SELECT name FROM sales_branch UNION SELECT name FROM budgets_branch
)
SELECT stitch_keys.name, sales_branch.total_sales, budgets_branch.budget_total
FROM stitch_keys
LEFT JOIN sales_branch ON stitch_keys.name IS NOT DISTINCT FROM sales_branch.name
LEFT JOIN budgets_branch ON stitch_keys.name IS NOT DISTINCT FROM budgets_branch.name
