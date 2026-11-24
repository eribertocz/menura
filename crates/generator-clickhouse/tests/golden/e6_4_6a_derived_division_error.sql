CASE WHEN toFloat64("ORDERS") = 0 THEN throwIf(ifNull(toFloat64("ORDERS") = 0, 0), 'RDER-01: division by zero in metric REVENUE_PER_ORDER') ELSE toFloat64("REVENUE") / toFloat64("ORDERS") END
