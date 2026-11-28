CREATE SCHEMA shop;
CREATE TABLE shop.sales (sale_id INTEGER NOT NULL, customer_id INTEGER, sold_on DATE, sold_at TIMESTAMPTZ);
INSERT INTO shop.sales VALUES (1, 1, DATE '2026-01-15', TIMESTAMPTZ '2026-01-15 10:00:00+00');
CREATE TABLE shop.customer_versions (customer_id INTEGER NOT NULL, segment VARCHAR NOT NULL, valid_from DATE NOT NULL, valid_to DATE);
INSERT INTO shop.customer_versions VALUES (1, 'RETAIL', DATE '2026-01-01', NULL);
