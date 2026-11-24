accurateCast(CASE WHEN "ORDERS" = 0 THEN throwIf(ifNull("ORDERS" = 0, 0), 'RDER-01: division by zero in metric UNITS_LEFT') ELSE toInt256("UNITS") % toInt256("ORDERS") END, 'Nullable(Int128)')
