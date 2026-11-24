SELECT
  ordered.month AS month,
  fiscal.label AS label,
  SUM(orders.amount) AS total_amount
FROM shop.orders AS orders
JOIN shop.calendar AS ordered ON orders.order_day = ordered.day
JOIN shop.calendar AS shipped ON orders.ship_day = shipped.day
JOIN shop.fiscal_years AS fiscal ON shipped.fiscal_year = fiscal.fiscal_year
GROUP BY ordered.month, fiscal.label
