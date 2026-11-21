CREATE TABLE shop.main.regions (region_id INTEGER NOT NULL, name VARCHAR);
INSERT INTO shop.main.regions VALUES (1, 'NORTH'), (2, 'SOUTH'), (3, 'EAST'), (4, 'WEST');
CREATE TABLE shop.main.invoices (invoice_id INTEGER NOT NULL, region_id INTEGER, billed DECIMAL(10, 2) NOT NULL, collected DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.main.invoices VALUES (1, 1, 100.00, 80.00), (2, 1, 100.00, 40.00), (3, 2, 0.00, 0.00), (4, 3, 50.00, 50.00), (5, 4, 10.00, 5.00);
CREATE TABLE shop.main.budgets (budget_id INTEGER NOT NULL, region_id INTEGER, amount DECIMAL(10, 2) NOT NULL);
INSERT INTO shop.main.budgets VALUES (1, 1, 200.00), (2, 2, 10.00), (3, 3, 25.00), (4, 4, 0.00);
