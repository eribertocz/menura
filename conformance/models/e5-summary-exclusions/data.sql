CREATE TABLE shop.main.orders (order_id INTEGER NOT NULL, order_date DATE NOT NULL, price DOUBLE NOT NULL, created_at TIMESTAMP WITH TIME ZONE NOT NULL);
INSERT INTO shop.main.orders VALUES (1, DATE '2026-10-05', 1.5, TIMESTAMPTZ '2026-10-05 10:00:00+00'), (2, DATE '2026-10-20', 2.25, TIMESTAMPTZ '2026-10-20 03:00:00+00'), (3, DATE '2026-11-02', 0.5, TIMESTAMPTZ '2026-11-02 12:00:00+00'), (4, DATE '2026-11-02', 4.0, TIMESTAMPTZ '2026-11-02 12:00:00+00');
CREATE TABLE shop.main.orders_price_by_month (order_month DATE NOT NULL, price_sum DOUBLE NOT NULL, order_count BIGINT NOT NULL);
INSERT INTO shop.main.orders_price_by_month VALUES (DATE '2026-10-01', 3.75, 2), (DATE '2026-11-01', 4.5, 2);
CREATE TABLE shop.main.orders_created_by_month (order_month DATE NOT NULL, created_count BIGINT NOT NULL, order_count BIGINT NOT NULL);
INSERT INTO shop.main.orders_created_by_month VALUES (DATE '2026-10-01', 2, 2), (DATE '2026-11-01', 2, 2);
