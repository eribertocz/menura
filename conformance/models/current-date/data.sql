CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, status VARCHAR NOT NULL, amount INTEGER NOT NULL, ordered_on DATE NOT NULL);
INSERT INTO shop.main.orders VALUES (1, 'OPEN', 3, DATE '2026-11-01'), (2, 'OPEN', 2, DATE '2026-11-14'), (3, 'CLOSED', 5, DATE '2026-12-01');
