CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL, city VARCHAR);
INSERT INTO shop.main.customers VALUES (1, 'NORTH', 'Oslo'), (2, 'NORTH', 'Bergen'), (2, 'NORTH', 'Bergen'), (3, 'NORTH', NULL), (3, 'NORTH', NULL), (4, 'SOUTH', 'Rome'), (4, 'NORTH', 'Rome');
CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, customer_id BIGINT, status VARCHAR NOT NULL, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.main.orders VALUES (100, 1, 'paid', 10.00), (101, 1, 'void', 20.00), (102, 2, 'paid', 5.00), (103, 3, 'paid', 3.00), (104, 4, 'paid', 40.00), (105, NULL, 'paid', 80.00), (106, 99, 'paid', 160.00);
CREATE TABLE shop.main.refunds (refund_id INTEGER NOT NULL, customer_id INTEGER, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.main.refunds VALUES (1, 2, 1.00), (2, 3, 2.00), (3, 4, 4.00);
