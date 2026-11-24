CREATE SCHEMA shop;
CREATE TABLE shop.customers (customer_id INTEGER, region VARCHAR NOT NULL);
INSERT INTO shop.customers VALUES
  (1, 'NORTH'),
  (2, 'SOUTH'),
  (2, 'EAST'),
  (NULL, 'WEST'),
  (NULL, 'WEST');
CREATE TABLE shop.invoices (invoice_id INTEGER NOT NULL, customer_id INTEGER, store_code VARCHAR, amount DECIMAL(10,2) NOT NULL);
INSERT INTO shop.invoices VALUES
  (10, 1, 'LPZ', 100.00),
  (11, 2, 'LPZ', 50.00),
  (12, 3, 'SCZ', 25.00),
  (13, NULL, NULL, 10.00),
  (14, 3, 'CBB', 5.00);
CREATE TABLE shop.stores (store_code VARCHAR NOT NULL, city VARCHAR NOT NULL);
INSERT INTO shop.stores VALUES
  ('LPZ', 'La Paz'),
  ('SCZ', 'Santa Cruz');
