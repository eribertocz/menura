CREATE TABLE shop.main.invoices (invoice_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, invoice_date DATE NOT NULL, billed INTEGER NOT NULL);
INSERT INTO shop.main.invoices VALUES (1, 1, DATE '2025-03-10', 100), (2, 1, DATE '2026-02-01', 50), (3, 2, DATE '2026-05-20', 200);
CREATE TABLE shop.main.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.main.customers VALUES (1, 'NORTH'), (2, 'SOUTH');
