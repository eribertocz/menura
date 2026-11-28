CREATE SCHEMA shop;
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, segment VARCHAR NOT NULL, active BOOLEAN NOT NULL, valid_from DATE NOT NULL, valid_to TIMESTAMP_S);
INSERT INTO shop.customers VALUES (1, 'RETAIL', true, DATE '2026-01-01', NULL), (2, 'RETAIL', false, DATE '2026-01-01', TIMESTAMP_S '2026-03-31 00:00:00'), (3, 'CORPORATE', true, DATE '2026-02-01', NULL);
CREATE TABLE shop.account_months (account_id INTEGER NOT NULL, period_start DATE NOT NULL, balance INTEGER NOT NULL);
INSERT INTO shop.account_months VALUES (10, DATE '2026-01-01', 100), (10, DATE '2026-02-01', 120), (11, DATE '2026-01-01', 50);
