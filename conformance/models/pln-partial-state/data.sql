CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, order_date DATE NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, 10, DATE '2026-10-05', 100), (2, 10, DATE '2026-10-20', 50), (3, 20, DATE '2026-10-20', 200);
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES (10, 'NORTH'), (20, 'SOUTH'), (30, 'EAST');
