CREATE TABLE shop.main.balances (customer_id INTEGER NOT NULL, period DATE NOT NULL, balance INTEGER NOT NULL);
INSERT INTO shop.main.balances VALUES (1, DATE '2026-01-01', 100), (3, DATE '2026-01-01', 50);
CREATE TABLE shop.main.members (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL, valid_from DATE NOT NULL, valid_to DATE);
INSERT INTO shop.main.members VALUES (1, 'NORTH', DATE '2026-01-01', NULL), (2, 'SOUTH', DATE '2026-01-01', NULL), (3, 'NORTH', DATE '2026-01-01', NULL);
