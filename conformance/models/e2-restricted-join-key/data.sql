CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.orders VALUES (1, 1, 10.00), (2, 1, 5.50), (3, 2, 7.25);
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, segment VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES (1, 'RETAIL'), (2, 'WHOLESALE');
