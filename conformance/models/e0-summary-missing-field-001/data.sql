CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, region VARCHAR NOT NULL, order_date DATE NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, 'NORTH', DATE '2026-11-01', 100), (2, 'NORTH', DATE '2026-11-01', 50), (3, 'SOUTH', DATE '2026-11-02', 200);
CREATE TABLE shop.orders_daily (order_date DATE NOT NULL, region VARCHAR NOT NULL, total_amount BIGINT NOT NULL, order_count BIGINT NOT NULL);
INSERT INTO shop.orders_daily VALUES (DATE '2026-11-01', 'NORTH', 150, 2), (DATE '2026-11-02', 'SOUTH', 200, 1);
