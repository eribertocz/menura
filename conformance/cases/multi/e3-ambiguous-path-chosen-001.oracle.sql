SELECT
  ordered.month AS month,
  SUM(orders.amount) AS total_amount
FROM shop.main.orders AS orders
JOIN shop.main.calendar AS ordered ON orders.order_day = ordered.day
GROUP BY ordered.month
