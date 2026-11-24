CREATE SCHEMA shop;
CREATE TABLE shop.invoices (invoice_id INTEGER NOT NULL, customer_id INTEGER NOT NULL, billed INTEGER NOT NULL, collected INTEGER NOT NULL);
INSERT INTO shop.invoices VALUES (1, 1, 100, 80), (2, 1, 50, 50), (3, 2, 200, 100);
CREATE TABLE shop.customers (customer_id INTEGER NOT NULL, region VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES (1, 'NORTH'), (2, 'SOUTH');
