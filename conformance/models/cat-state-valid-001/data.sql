CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, order_date DATE NOT NULL, status VARCHAR NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, DATE '2026-09-03', 'OPEN', 10), (2, DATE '2026-09-20', 'OPEN', 5), (3, DATE '2026-11-02', 'CLOSED', 7);
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES (1, 'NORTH');
