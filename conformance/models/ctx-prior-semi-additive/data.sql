CREATE TABLE shop.main.balances (customer_id INTEGER NOT NULL, period DATE NOT NULL, balance INTEGER, paid INTEGER NOT NULL);
INSERT INTO shop.main.balances VALUES (1, DATE '2026-01-01', 100, 10), (1, DATE '2026-02-01', 110, 10), (1, DATE '2026-03-01', 120, 10), (1, DATE '2026-04-01', 130, 10), (2, DATE '2026-01-01', 50, 5), (2, DATE '2026-02-01', 60, 5), (3, DATE '2026-01-01', 30, 3), (3, DATE '2026-02-01', 40, 3), (4, DATE '2025-12-01', 70, 7), (5, DATE '2026-02-01', 20, 2), (5, DATE '2026-03-01', 40, 2);
CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, category VARCHAR NOT NULL);
INSERT INTO shop.main.customers VALUES (1, 'A'), (2, 'B'), (3, 'A'), (4, 'A'), (5, 'A');
CREATE TABLE shop.main.members (customer_id INTEGER NOT NULL, valid_from DATE NOT NULL, valid_to DATE);
INSERT INTO shop.main.members VALUES (1, DATE '2026-01-01', NULL), (2, DATE '2026-01-01', NULL), (3, DATE '2026-01-01', DATE '2026-02-28'), (4, DATE '2025-12-01', NULL), (5, DATE '2026-02-01', NULL);
CREATE TABLE shop.main.stock (warehouse VARCHAR NOT NULL, product VARCHAR NOT NULL, quantity INTEGER NOT NULL);
INSERT INTO shop.main.stock VALUES ('W1', 'P1', 5), ('W2', 'P1', 8), ('W1', 'P2', 3), ('W2', 'P2', 1);
