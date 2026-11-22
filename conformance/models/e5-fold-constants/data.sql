CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, segment VARCHAR NOT NULL);
INSERT INTO shop.main.customers VALUES (1, 'retail'), (2, 'wholesale');
CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.main.orders VALUES (1, 1, 10.00), (2, 1, 5.50), (3, 2, 20.00);
