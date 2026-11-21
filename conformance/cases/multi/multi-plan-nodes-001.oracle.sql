WITH visible_ids AS (
  SELECT customer_id
  FROM shop.main.customers
  GROUP BY customer_id
  HAVING bool_and(region IN ('NORTH'))
),
visible_customers AS (
  SELECT DISTINCT customers.customer_id, customers.city
  FROM shop.main.customers AS customers
  JOIN visible_ids ON customers.customer_id = visible_ids.customer_id
),
order_totals AS (
  SELECT visible_customers.city AS city, SUM(orders.amount) AS order_total
  FROM shop.main.orders AS orders
  JOIN visible_customers ON orders.customer_id = visible_customers.customer_id
  WHERE orders.status <> 'void'
  GROUP BY visible_customers.city
),
refund_totals AS (
  SELECT visible_customers.city AS city, SUM(refunds.amount) AS refund_total
  FROM shop.main.refunds AS refunds
  JOIN visible_customers ON refunds.customer_id = visible_customers.customer_id
  GROUP BY visible_customers.city
),
cities AS (
  SELECT city FROM order_totals
  UNION
  SELECT city FROM refund_totals
)
SELECT cities.city AS city, refund_totals.refund_total AS refund_total, order_totals.order_total AS order_total
FROM cities
LEFT JOIN refund_totals ON cities.city IS NOT DISTINCT FROM refund_totals.city
LEFT JOIN order_totals ON cities.city IS NOT DISTINCT FROM order_totals.city
ORDER BY cities.city ASC NULLS FIRST
LIMIT 2
