WITH refunds_branch AS (
  SELECT SUM(amount) AS refunded FROM shop.main.refunds
), orders_branch AS (
  SELECT SUM(amount) AS ordered FROM shop.main.orders
)
SELECT CASE WHEN orders_branch.ordered = 0 THEN NULL
    ELSE CAST(refunds_branch.refunded AS DOUBLE) / CAST(orders_branch.ordered AS DOUBLE) END AS refund_rate
FROM refunds_branch CROSS JOIN orders_branch
