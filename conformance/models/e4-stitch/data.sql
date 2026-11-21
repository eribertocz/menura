CREATE TABLE shop.main.regions (region_id INTEGER NOT NULL, name VARCHAR);
INSERT INTO shop.main.regions VALUES (1, 'NORTH'), (2, 'SOUTH'), (3, 'EAST'), (4, NULL);
CREATE TABLE shop.main.sales (sale_id INTEGER NOT NULL, region_id INTEGER, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.main.sales VALUES (1, 1, 10.00), (2, 1, 5.00), (3, 2, 7.00), (4, 4, 3.00), (5, NULL, 2.00);
CREATE TABLE shop.main.budgets (budget_id INTEGER NOT NULL, region_id INTEGER, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.main.budgets VALUES (1, 1, 20.00), (2, 3, 8.00), (3, NULL, 4.00);
