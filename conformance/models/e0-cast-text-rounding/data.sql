CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, amount_text VARCHAR NOT NULL);
INSERT INTO shop.orders VALUES (1, '2.25'), (2, '2.25'), (3, '0.05'), (4, '2.24');
