CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, status VARCHAR NOT NULL, amount DECIMAL(10, 2) NOT NULL, order_date DATE NOT NULL);
INSERT INTO shop.main.orders VALUES
  (1, 1, 'open', 10.00, DATE '2026-01-05'),
  (2, 1, 'shipped', 20.00, DATE '2026-01-20'),
  (3, 2, 'open', 30.00, DATE '2026-02-10');
CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, region_id INTEGER NOT NULL, customer_name VARCHAR NOT NULL);
INSERT INTO shop.main.customers VALUES (1, 1, 'Ada'), (2, 2, 'Grace'), (3, 2, 'Linus');
CREATE TABLE shop.main.regions (region_id INTEGER NOT NULL, region_name VARCHAR NOT NULL);
INSERT INTO shop.main.regions VALUES (1, 'NORTH'), (2, 'SOUTH');
CREATE TABLE shop.main.returns (return_id INTEGER NOT NULL, order_id INTEGER NOT NULL, reason VARCHAR NOT NULL, refund DECIMAL(10, 2) NOT NULL, return_date DATE NOT NULL);
INSERT INTO shop.main.returns VALUES (1, 2, 'late', 20.00, DATE '2026-01-25');
CREATE TABLE shop.main.warehouses (warehouse_id INTEGER NOT NULL, city VARCHAR NOT NULL);
INSERT INTO shop.main.warehouses VALUES (1, 'Lima'), (2, 'Quito');
CREATE TABLE shop.main.accounts (account_id INTEGER NOT NULL, plan_name VARCHAR NOT NULL);
INSERT INTO shop.main.accounts VALUES (1, 'basic'), (2, 'premium');
CREATE TABLE shop.main.profiles (account_id INTEGER NOT NULL, segment VARCHAR NOT NULL);
INSERT INTO shop.main.profiles VALUES (1, 'retail'), (2, 'wholesale');
