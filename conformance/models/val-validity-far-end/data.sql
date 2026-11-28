CREATE SCHEMA shop;
CREATE TABLE shop.balances (customer_id INTEGER NOT NULL, period DATE NOT NULL, balance INTEGER);
INSERT INTO shop.balances VALUES (1, DATE '2026-03-01', 130), (2, DATE '2026-01-01', 50), (3, DATE '2026-03-01', 70);
CREATE TABLE shop.members (customer_id INTEGER NOT NULL, valid_from DATE NOT NULL, valid_to DATE);
INSERT INTO shop.members VALUES (1, DATE '2026-01-01', NULL), (2, DATE '2026-01-01', DATE '9999-12-31'), (3, DATE '2026-01-01', NULL);
