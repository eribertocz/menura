CREATE SCHEMA shop;
CREATE TABLE shop.balances (customer_id INTEGER NOT NULL, period DATE NOT NULL, balance INTEGER, paid INTEGER NOT NULL);
INSERT INTO shop.balances VALUES (1, DATE '2026-01-01', 100, 10), (1, DATE '2026-02-01', 110, 10), (2, DATE '2026-01-01', 50, 5), (2, DATE '2026-02-01', 60, 5), (3, DATE '2026-02-01', 30, 3), (6, DATE '2026-02-01', 90, 9), (7, DATE '2026-01-01', 20, 2), (7, DATE '2026-02-01', 25, 2);
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, category VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES (1, 'A'), (2, 'B'), (3, 'A'), (6, 'C');
CREATE TABLE shop.members (customer_id INTEGER NOT NULL, valid_from DATE NOT NULL, valid_to DATE);
INSERT INTO shop.members VALUES (1, DATE '2026-01-01', NULL), (2, DATE '2026-01-01', NULL), (3, DATE '2026-01-01', NULL), (7, DATE '2026-01-01', NULL);
