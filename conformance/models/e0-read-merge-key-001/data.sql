CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, status VARCHAR NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, 'OPEN', 10), (2, 'OPEN', 5), (3, 'CLOSED', 7);
CREATE TABLE shop.balances (branch VARCHAR NOT NULL, customer_id INTEGER NOT NULL, balance INTEGER);
INSERT INTO shop.balances VALUES ('NORTH', 1, 100), ('SOUTH', 1, NULL);
