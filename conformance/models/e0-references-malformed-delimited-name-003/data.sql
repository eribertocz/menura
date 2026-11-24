CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, status VARCHAR NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, 1, 'OPEN', 10), (2, 2, 'OPEN', 5), (3, 1, 'CLOSED', 7);
