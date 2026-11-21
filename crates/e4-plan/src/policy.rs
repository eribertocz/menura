//! What a row filter reads (E4-6.9b).

use std::convert::Infallible;

use menura_model::expression::Expression;

use crate::rewrite::rewrite;

/// Whether `filter`, which holds the conditions of the applicable policies only (E4-6.2),
/// reads `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP`.
pub(crate) fn reads_now(filter: &Expression) -> bool {
    let mut found = false;
    let visited: Result<Expression, Infallible> = rewrite(filter, &mut |node| {
        found |= matches!(
            node,
            Expression::CurrentDate | Expression::CurrentTime | Expression::CurrentTimestamp
        );
        Ok(None)
    });
    drop(visited);
    found
}
