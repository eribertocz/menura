//! Positions and counts written as integer literals (E1-4 "GROUP BY", "ORDER BY", "LIMIT").

use std::ops::Range;

use super::lexer::{Token, TokenKind};

/// The integer written by the tokens `range`, when they are one number of digits only; a
/// number too large for 64 bits reads as [`u64::MAX`].
pub(super) fn integer_literal(sql: &str, tokens: &[Token], range: Range<usize>) -> Option<u64> {
    let [token] = tokens.get(range)? else {
        return None;
    };
    let text = token.text(sql);
    let digits_only =
        token.kind == TokenKind::Number && text.bytes().all(|byte| byte.is_ascii_digit());
    digits_only.then(|| text.parse().unwrap_or(u64::MAX))
}
