CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, status VARCHAR NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.main.orders VALUES (1, 1, 'OPEN', 10), (2, 1, 'CLOSED', 20), (3, 2, 'OPEN', 5), (4, 3, 'CLOSED', 7);
CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL, age INTEGER NOT NULL);
INSERT INTO shop.main.customers VALUES (1, 'north', 30), (2, 'south', 50), (3, 'north', 40);
