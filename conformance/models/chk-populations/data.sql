CREATE SCHEMA shop;
CREATE TABLE shop.memberships (customer_id INTEGER, valid_from DATE, valid_to DATE);
INSERT INTO shop.memberships VALUES
  (1, DATE '2026-01-01', DATE '2026-01-31'),
  (1, DATE '2026-01-31', DATE '2026-02-28'),
  (2, DATE '2026-03-10', DATE '2026-03-01'),
  (2, DATE '2026-03-01', NULL),
  (3, DATE '2026-01-01', DATE '2026-01-31'),
  (3, DATE '2026-02-01', NULL),
  (4, NULL, NULL),
  (4, DATE '2026-01-01', NULL),
  (5, DATE '2026-01-01', NULL),
  (5, DATE '2026-01-01', NULL),
  (NULL, DATE '2026-01-01', NULL),
  (NULL, DATE '2026-01-01', NULL);
CREATE TABLE shop.stamped_memberships (customer_id INTEGER NOT NULL, valid_from TIMESTAMPTZ NOT NULL, valid_to TIMESTAMPTZ);
INSERT INTO shop.stamped_memberships VALUES
  (1, TIMESTAMPTZ '2026-03-01 02:00:00+00', TIMESTAMPTZ '2026-03-31 23:00:00+00'),
  (1, TIMESTAMPTZ '2026-04-01 02:00:00+00', NULL);
CREATE TABLE shop.roster (customer_id INTEGER, period_at DATE);
INSERT INTO shop.roster VALUES
  (1, DATE '2026-03-01'),
  (1, DATE '2026-03-15'),
  (2, DATE '2026-03-01'),
  (2, DATE '2026-04-01'),
  (3, NULL),
  (3, NULL),
  (NULL, DATE '2026-03-01'),
  (NULL, DATE '2026-03-01');
CREATE TABLE shop.balances (customer_id INTEGER, period DATE, snapshot INTEGER, balance INTEGER NOT NULL);
INSERT INTO shop.balances VALUES
  (1, DATE '2026-01-15', 1, 10),
  (1, DATE '2026-01-20', 1, 20),
  (1, DATE '2026-03-10', 2, 30),
  (2, DATE '2026-04-05', 3, 40),
  (4, DATE '2026-03-01', 4, 50),
  (NULL, DATE '2026-02-10', 5, 60),
  (5, NULL, NULL, 70);
