CREATE TABLE shop.main.balances (customer_id INTEGER NOT NULL, period DATE NOT NULL, balance INTEGER);
INSERT INTO shop.main.balances VALUES (1, DATE '2025-12-01', NULL), (2, DATE '2026-02-01', 10);
CREATE TABLE shop.main.members (customer_id INTEGER NOT NULL, valid_from DATE NOT NULL, valid_to DATE);
INSERT INTO shop.main.members VALUES (1, DATE '2025-12-01', NULL);
