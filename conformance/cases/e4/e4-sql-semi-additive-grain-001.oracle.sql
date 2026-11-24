WITH last_periods AS (
  SELECT CAST(DATE_TRUNC('year', period) AS DATE) AS period_year, MAX(period) AS last_period
  FROM shop.balances
  GROUP BY 1
)
SELECT
  last_periods.period_year AS period_year,
  SUM(balances.balance) AS balance
FROM last_periods
JOIN shop.balances AS balances ON balances.period = last_periods.last_period
GROUP BY last_periods.period_year
ORDER BY period_year ASC NULLS LAST
