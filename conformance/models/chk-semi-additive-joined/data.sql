CREATE TABLE shop.main.accounts (account_id INTEGER NOT NULL, customer_id INTEGER);
INSERT INTO shop.main.accounts VALUES (1, 10), (2, 10), (3, 20);
CREATE TABLE shop.main.balances (account_id INTEGER, snapshot INTEGER, balance INTEGER NOT NULL);
INSERT INTO shop.main.balances VALUES (1, 1, 100), (2, 1, 50), (3, 1, 70), (1, 2, 110);
