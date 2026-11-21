SELECT sales_branch.total_sales, budgets_branch.budget_count
FROM (SELECT SUM(sales.amount) AS total_sales
  FROM shop.main.sales AS sales) AS sales_branch
CROSS JOIN (SELECT COUNT(budgets.budget_id) AS budget_count
  FROM shop.main.budgets AS budgets) AS budgets_branch
