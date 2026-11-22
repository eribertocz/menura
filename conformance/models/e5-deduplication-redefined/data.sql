CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.main.orders VALUES (1, 1, 10), (2, 1, 5), (3, 2, 7);
CREATE TABLE shop.main.clients (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.main.clients VALUES (1, 'NORTH'), (2, 'SOUTH');
