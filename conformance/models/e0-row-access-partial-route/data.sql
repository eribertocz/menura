CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, store_id INTEGER NOT NULL, supplier_id INTEGER NOT NULL, region VARCHAR NOT NULL, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.orders VALUES (10, 1, 1, 'NORTH', 1.00), (11, 1, 2, 'NORTH', 2.00);
CREATE TABLE shop.stores (store_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.stores VALUES (1, 'NORTH');
CREATE TABLE shop.suppliers (supplier_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.suppliers VALUES (1, 'NORTH'), (2, 'SOUTH');
