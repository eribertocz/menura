CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, region VARCHAR NOT NULL, order_date DATE NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, 'NORTH', DATE '2026-10-05', 100), (2, 'NORTH', DATE '2026-10-20', 50), (3, 'SOUTH', DATE '2026-10-20', 200), (4, 'NORTH', DATE '2026-11-01', 30), (5, 'SOUTH', DATE '2026-11-02', 70), (6, 'SOUTH', DATE '2026-11-02', 10);
CREATE TABLE shop.orders_by_month (order_month DATE NOT NULL, amount_sum BIGINT NOT NULL, amount_count BIGINT NOT NULL, order_count BIGINT NOT NULL);
INSERT INTO shop.orders_by_month VALUES (DATE '2026-10-01', 350, 3, 3), (DATE '2026-11-01', 110, 3, 3);
