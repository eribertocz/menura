CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, 1, 10), (2, 1, 5), (3, 2, 7);
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES (1, 'NORTH'), (2, 'SOUTH');
