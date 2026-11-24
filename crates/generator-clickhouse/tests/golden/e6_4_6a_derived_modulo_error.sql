CASE WHEN "ORDERS" = 0 THEN throwIf(ifNull("ORDERS" = 0, 0), 'RDER-01: division by zero in metric UNITS_LEFT') ELSE toInt128("UNITS") % toInt128("ORDERS") END
