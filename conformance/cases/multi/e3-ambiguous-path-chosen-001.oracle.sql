SELECT
  ordered.month AS month,
  SUM(orders.amount) AS total_amount
FROM shop.orders AS orders
JOIN shop.calendar AS ordered ON orders.order_day = ordered.day
GROUP BY ordered.month
