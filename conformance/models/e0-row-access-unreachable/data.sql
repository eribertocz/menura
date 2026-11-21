CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.main.customers VALUES (1, 'NORTH'), (2, 'SOUTH');
CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, region VARCHAR NOT NULL, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.main.orders VALUES (10, 1, 'NORTH', 1.00), (11, 2, 'SOUTH', 2.00);
