CREATE SCHEMA shop;
CREATE TABLE shop.payments (payment_id INTEGER NOT NULL, "Amount" INTEGER NOT NULL);
INSERT INTO shop.payments VALUES (1, 4), (2, 6);
