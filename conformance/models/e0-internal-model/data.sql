CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, status VARCHAR NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.main.orders VALUES (1, 10, 'open', 100), (2, 10, 'closed', 50), (3, 20, 'open', 200);
CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.main.customers VALUES (10, 'NORTH'), (20, 'SOUTH');
