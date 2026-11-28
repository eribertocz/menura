CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, region VARCHAR NOT NULL, volume INTEGER NOT NULL);
INSERT INTO shop.orders VALUES (1, 'NORTH', 5), (2, 'NORTH', 5), (3, 'SOUTH', 8);
