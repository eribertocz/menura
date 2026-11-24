WITH sales_branch AS (
  SELECT regions.name AS name, SUM(sales.amount) AS total_sales
  FROM shop.sales AS sales
  LEFT JOIN shop.regions AS regions ON sales.region_id = regions.region_id
  GROUP BY regions.name
), budgets_branch AS (
  SELECT regions.name AS name, COUNT(budgets.budget_id) AS budget_count
  FROM shop.budgets AS budgets
  LEFT JOIN shop.regions AS regions ON budgets.region_id = regions.region_id
  GROUP BY regions.name
), stitch_keys AS (
  SELECT name FROM sales_branch UNION SELECT name FROM budgets_branch
)
SELECT stitch_keys.name, COALESCE(budgets_branch.budget_count, 0) AS budget_count, sales_branch.total_sales
FROM stitch_keys
LEFT JOIN sales_branch ON stitch_keys.name IS NOT DISTINCT FROM sales_branch.name
LEFT JOIN budgets_branch ON stitch_keys.name IS NOT DISTINCT FROM budgets_branch.name
