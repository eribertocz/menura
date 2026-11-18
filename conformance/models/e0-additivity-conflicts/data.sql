CREATE TABLE shop.main.debt (customer_id INTEGER NOT NULL, period DATE NOT NULL, balance INTEGER NOT NULL);
INSERT INTO shop.main.debt VALUES (1, DATE '2026-01-01', 100), (1, DATE '2026-02-01', 80), (2, DATE '2026-01-01', 50), (3, DATE '2026-02-01', 30);
CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, category VARCHAR NOT NULL, valid_from DATE NOT NULL, valid_to DATE);
INSERT INTO shop.main.customers VALUES (1, 'RETAIL', DATE '2026-01-01', NULL), (2, 'RETAIL', DATE '2026-01-01', NULL), (3, 'CORPORATE', DATE '2026-01-01', NULL);
