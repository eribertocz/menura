SELECT sales_branch.total_sales, budgets_branch.budget_count
FROM (SELECT SUM(sales.amount) AS total_sales
  FROM shop.sales AS sales) AS sales_branch
CROSS JOIN (SELECT COUNT(budgets.budget_id) AS budget_count
  FROM shop.budgets AS budgets) AS budgets_branch
