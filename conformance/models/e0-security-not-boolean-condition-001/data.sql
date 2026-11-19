CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, region VARCHAR NOT NULL, amount INTEGER NOT NULL, unit_cost INTEGER NOT NULL);
INSERT INTO shop.main.orders VALUES (1, 'NORTH', 100, 60), (2, 'NORTH', 50, 30), (3, 'SOUTH', 200, 120);
CREATE TABLE shop.main.notes (note_id INTEGER NOT NULL, body VARCHAR NOT NULL);
INSERT INTO shop.main.notes VALUES (1, 'first'), (2, 'second');
