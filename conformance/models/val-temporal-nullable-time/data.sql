CREATE SCHEMA shop;
CREATE TABLE shop.sales (sale_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, sold_on DATE, amount INTEGER NOT NULL);
INSERT INTO shop.sales VALUES (1, 1, DATE '2026-01-15', 10), (2, 1, DATE '2026-02-01', 20), (3, 1, DATE '2100-06-01', 40), (4, 2, DATE '2026-03-10', 80), (5, 2, NULL, 160);
CREATE TABLE shop.customer_versions (customer_id INTEGER NOT NULL, segment VARCHAR NOT NULL, valid_from DATE NOT NULL, valid_to DATE);
INSERT INTO shop.customer_versions VALUES (1, 'RETAIL', DATE '2026-01-01', DATE '2026-02-01'), (1, 'CORPORATE', DATE '2026-02-01', DATE '2099-12-31'), (2, 'RETAIL', DATE '2026-03-01', DATE '2026-04-01');
