SELECT ordered.month AS month, fiscal.label AS label, SUM(orders.amount) AS total_amount
FROM shop.main.orders AS orders
LEFT JOIN shop.main.calendar AS ordered ON orders.order_day = ordered.day
LEFT JOIN shop.main.calendar AS shipped ON orders.ship_day = shipped.day
LEFT JOIN shop.main.fiscal_years AS fiscal ON shipped.fiscal_year = fiscal.fiscal_year
GROUP BY ordered.month, fiscal.label;
