CREATE SCHEMA shop;
CREATE TABLE shop.orders (order_id INTEGER NOT NULL, customer_id BIGINT NOT NULL, status VARCHAR NOT NULL, quantity INTEGER NOT NULL, price DECIMAL(10,2) NOT NULL, rate DECIMAL(30,20) NOT NULL, discount DOUBLE, payload STRUCT(a INTEGER), ordered_on DATE NOT NULL, ordered_at TIMESTAMP, note VARCHAR);
INSERT INTO shop.orders VALUES (1, 1, 'OPEN', 3, 10.50, 1.5, 0.5, {'a': 1}, DATE '2026-11-01', TIMESTAMP '2026-11-01 10:00:00', NULL), (2, 2, 'OPEN', 2, 4.25, 2.0, NULL, {'a': 2}, DATE '2026-11-02', NULL, 'gift'), (3, 1, 'CLOSED', 5, 7.00, 1.0, 1.5, NULL, DATE '2026-11-03', TIMESTAMP '2026-11-03 09:30:00', NULL);
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, customer_code VARCHAR NOT NULL, name VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES (1, 'C1', 'Ada'), (2, 'C2', 'Grace');
