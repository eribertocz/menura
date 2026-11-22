CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, status VARCHAR NOT NULL, amount INTEGER NOT NULL);
INSERT INTO shop.main.orders VALUES (1, 1, 'open', 100), (2, 2, 'closed', 300), (3, 3, 'open', 200), (4, 3, 'void', 1000), (5, 4, 'open', 30), (6, 4, 'closed', -30), (7, 9, 'open', 50);
CREATE TABLE shop.main.returns (return_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, refund INTEGER NOT NULL);
INSERT INTO shop.main.returns VALUES (1, 1, 20), (2, 3, 40);
CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.main.customers VALUES (1, 'NORTH'), (2, 'NORTH'), (3, 'SOUTH'), (4, 'WEST');
