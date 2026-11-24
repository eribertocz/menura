CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.orders VALUES (1, 100.00), (2, 60.00), (3, 40.00);
CREATE TABLE shop.refunds (refund_id INTEGER NOT NULL, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.refunds VALUES (1, 30.00), (2, 20.00);
