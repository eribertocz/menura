//! The text functions and `LIKE` (E0F-10.2), with the semantics of E0F-10 where `ClickHouse`'s
//! defaults differ. Lengths and positions count characters, so the UTF-8 functions are used.

use menura_model::expression::{Expression, LikeCase, ScalarFunction};

use super::ExpressionWriter;
use crate::error::{GenerateError, InvariantViolation};

impl ExpressionWriter<'_, '_> {
    /// The call of a text `function`, before the cast to its result type.
    pub(super) fn text_function(
        &mut self,
        function: ScalarFunction,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        match function {
            ScalarFunction::Concat => self.concat(arguments),
            ScalarFunction::Length => self.unary_call("lengthUTF8", arguments),
            ScalarFunction::Lower => self.unary_call("lowerUTF8", arguments),
            ScalarFunction::Upper => self.unary_call("upperUTF8", arguments),
            ScalarFunction::Trim => self.trim("BOTH", arguments),
            ScalarFunction::LeftTrim => self.trim("LEADING", arguments),
            ScalarFunction::RightTrim => self.trim("TRAILING", arguments),
            ScalarFunction::Left => self.end_characters("leftUTF8", arguments),
            ScalarFunction::Right => self.end_characters("rightUTF8", arguments),
            ScalarFunction::Substring => self.substring(arguments),
            ScalarFunction::Replace => self.replace(arguments),
            ScalarFunction::SplitPart => self.split_part(arguments),
            ScalarFunction::Position => self.position(arguments),
            ScalarFunction::Contains => self.contains(arguments),
            ScalarFunction::StartsWith => self.call("startsWith", arguments),
            ScalarFunction::EndsWith => self.call("endsWith", arguments),
            _ => Err(self.error(InvariantViolation::MalformedFunctionCall)),
        }
    }

    /// `x [NOT] LIKE p` or `x [NOT] ILIKE p`. `ClickHouse` reads a backslash in a pattern as
    /// an escape character, which E0F-10.2 does not have, so every backslash of the pattern is
    /// doubled first.
    pub(super) fn like(
        &mut self,
        operand: &Expression,
        pattern: &Expression,
        case: LikeCase,
        negated: bool,
    ) -> Result<String, GenerateError> {
        let operand = self.operand(operand)?;
        let pattern = self.write(pattern)?;
        let keyword = match (case, negated) {
            (LikeCase::Sensitive, false) => "LIKE",
            (LikeCase::Sensitive, true) => "NOT LIKE",
            (LikeCase::Insensitive, false) => "ILIKE",
            (LikeCase::Insensitive, true) => "NOT ILIKE",
        };
        Ok(format!(
            "{operand} {keyword} replaceAll({pattern}, '\\\\', '\\\\\\\\')"
        ))
    }

    /// `name(a, b, …)`.
    pub(super) fn call(
        &mut self,
        name: &str,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let written = arguments
            .iter()
            .map(|argument| self.write(argument))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!("{name}({})", written.join(", ")))
    }

    /// `name(x)` for a function of one argument.
    pub(super) fn unary_call(
        &mut self,
        name: &str,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let [value] = self.exactly(arguments)?;
        Ok(format!("{name}({})", self.write(value)?))
    }

    /// `a || b || …`, NULL when any argument is NULL.
    fn concat(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        if arguments.is_empty() {
            return Err(self.error(InvariantViolation::MalformedFunctionCall));
        }
        let written = arguments
            .iter()
            .map(|argument| self.operand(argument))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(written.join(" || "))
    }

    /// `trim(<side> ' ' FROM s)`: spaces (U+0020) only.
    fn trim(&mut self, side: &str, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [text] = self.exactly(arguments)?;
        Ok(format!("trim({side} ' ' FROM {})", self.write(text)?))
    }

    /// `leftUTF8(s, n)` or `rightUTF8(s, n)`, NULL for `n < 0`, where `ClickHouse` drops
    /// characters instead. `n` is clamped to the length of `s` before its cast to `Int64`, so a
    /// 128-bit count cannot overflow.
    fn end_characters(
        &mut self,
        name: &str,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let [text, count] = self.exactly(arguments)?;
        let text = self.write(text)?;
        let count = self.operand(count)?;
        Ok(format!(
            "{name}({text}, CASE WHEN {count} >= 0 THEN \
             CAST(least({count}, {}) AS Int64) END)",
            characters(&text)
        ))
    }

    /// The characters at positions `start` to `start + length − 1` that exist (ISO 9075):
    /// `ClickHouse` counts a start before 1 from the end of the text instead. A NULL argument
    /// gives NULL; positions are clamped to the text before their cast to `Int64`.
    fn substring(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let (text, start, length) = match arguments {
            [text, start] => (text, start, None),
            [text, start, length] => (text, start, Some(length)),
            _ => return Err(self.error(InvariantViolation::MalformedFunctionCall)),
        };
        let text = self.operand(text)?;
        let start = self.operand(start)?;
        let size = characters(&text);
        let first =
            format!("CAST(least(greatest(CAST({start} AS Int128), 1), {size} + 1) AS Int64)");
        let Some(length) = length else {
            return Ok(format!(
                "CASE WHEN {text} IS NULL OR {start} IS NULL THEN NULL \
                 ELSE substringUTF8({text}, {first}) END"
            ));
        };
        let length = self.operand(length)?;
        let count = format!(
            "CAST(least(greatest(CAST({start} AS Int128) + {length} \
             - greatest(CAST({start} AS Int128), 1), 0), {size}) AS Int64)"
        );
        Ok(format!(
            "CASE WHEN {text} IS NULL OR {start} IS NULL OR {length} IS NULL OR {length} < 0 \
             THEN NULL ELSE substringUTF8({text}, {first}, {count}) END"
        ))
    }

    /// `replaceAll(s, from, to)`, with `s` unchanged for an empty `from`.
    fn replace(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [text, from, to] = self.exactly(arguments)?;
        let text = self.operand(text)?;
        let from = self.operand(from)?;
        let to = self.operand(to)?;
        Ok(format!(
            "CASE WHEN {text} IS NULL OR {from} IS NULL OR {to} IS NULL THEN NULL \
             WHEN {from} = '' THEN {text} ELSE replaceAll({text}, {from}, {to}) END"
        ))
    }

    /// Part `n` of `s` split at `delimiter`, the literal `n` written in the SQL (E6-4.11), and
    /// empty text when there is no such part. An empty delimiter leaves `s` as its only part:
    /// `s` for `n = 1`, empty text otherwise. `splitByString` returns an array, which cannot be
    /// `Nullable`, so it reads its arguments through `ifNull` once NULL is ruled out.
    fn split_part(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [text, delimiter, number] = self.exactly(arguments)?;
        let number = self.literal_integer(number)?;
        let text = self.operand(text)?;
        let delimiter = self.operand(delimiter)?;
        let only_part = if number == 1 { text.as_str() } else { "''" };
        Ok(format!(
            "CASE WHEN {text} IS NULL OR {delimiter} IS NULL THEN NULL \
             WHEN {delimiter} = '' THEN {only_part} \
             ELSE splitByString(ifNull({delimiter}, ''), ifNull({text}, ''))[{number}] END"
        ))
    }

    /// `POSITION(a IN s)` as `positionUTF8(s, a)`, 1 when `a` is empty.
    fn position(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [needle, text] = self.exactly(arguments)?;
        let needle = self.operand(needle)?;
        let text = self.operand(text)?;
        Ok(format!(
            "CASE WHEN {needle} = '' AND {text} IS NOT NULL THEN 1 \
             ELSE positionUTF8({text}, {needle}) END"
        ))
    }

    /// `CONTAINS(s, a)` as `positionUTF8(s, a) > 0`, NULL when an argument is NULL.
    fn contains(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [text, needle] = self.exactly(arguments)?;
        let text = self.write(text)?;
        let needle = self.write(needle)?;
        Ok(format!("positionUTF8({text}, {needle}) > 0"))
    }
}

/// The number of characters of `text`, as an `Int64` so that it compares and combines with
/// signed integers without a missing common type.
fn characters(text: &str) -> String {
    format!("CAST(lengthUTF8({text}) AS Int64)")
}
