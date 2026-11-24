CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, status VARCHAR NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, 1, 'OPEN', 10), (2, 2, 'OPEN', 5), (3, 1, 'CLOSED', 7);
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, name VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES (1, 'Ada'), (2, 'Grace');
CREATE TABLE shop.labels (label_id INTEGER NOT NULL, name VARCHAR NOT NULL);
INSERT INTO shop.labels VALUES (1, 'red'), (2, 'blue');
