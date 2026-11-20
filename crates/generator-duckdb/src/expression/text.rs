//! The text functions and `LIKE` (E0F-10.2), with the semantics of E0F-10 where `DuckDB`'s
//! defaults differ.

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
            ScalarFunction::Length => self.unary_call("length", arguments),
            ScalarFunction::Lower => self.unary_call("lower", arguments),
            ScalarFunction::Upper => self.unary_call("upper", arguments),
            ScalarFunction::Trim => self.trim("trim", arguments),
            ScalarFunction::LeftTrim => self.trim("ltrim", arguments),
            ScalarFunction::RightTrim => self.trim("rtrim", arguments),
            ScalarFunction::Left => self.end_characters("left", arguments),
            ScalarFunction::Right => self.end_characters("right", arguments),
            ScalarFunction::Substring => self.substring(arguments),
            ScalarFunction::Replace => self.replace(arguments),
            ScalarFunction::SplitPart => self.split_part(arguments),
            ScalarFunction::Position => self.position(arguments),
            ScalarFunction::Contains => self.call("contains", arguments),
            ScalarFunction::StartsWith => self.call("prefix", arguments),
            ScalarFunction::EndsWith => self.call("suffix", arguments),
            _ => Err(self.error(InvariantViolation::MalformedFunctionCall)),
        }
    }

    /// `x [NOT] LIKE p` or `x [NOT] ILIKE p`, which `DuckDB` evaluates without an escape
    /// character unless `ESCAPE` is written (E0F-10.2).
    pub(super) fn like(
        &mut self,
        operand: &Expression,
        pattern: &Expression,
        case: LikeCase,
        negated: bool,
    ) -> Result<String, GenerateError> {
        let operand = self.operand(operand)?;
        let pattern = self.operand(pattern)?;
        let keyword = match (case, negated) {
            (LikeCase::Sensitive, false) => "LIKE",
            (LikeCase::Sensitive, true) => "NOT LIKE",
            (LikeCase::Insensitive, false) => "ILIKE",
            (LikeCase::Insensitive, true) => "NOT ILIKE",
        };
        Ok(format!("{operand} {keyword} {pattern}"))
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

    /// `a || b || …`, NULL when any argument is NULL; `DuckDB`'s `concat` skips NULLs.
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

    /// `trim(s, ' ')`: spaces (U+0020) only.
    fn trim(&mut self, name: &str, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [text] = self.exactly(arguments)?;
        Ok(format!("{name}({}, ' ')", self.write(text)?))
    }

    /// `left(s, n)` or `right(s, n)`, NULL for `n < 0`, where `DuckDB` drops characters
    /// instead.
    fn end_characters(
        &mut self,
        name: &str,
        arguments: &[Expression],
    ) -> Result<String, GenerateError> {
        let [text, count] = self.exactly(arguments)?;
        let text = self.write(text)?;
        let count = self.operand(count)?;
        Ok(format!(
            "{name}({text}, CASE WHEN {count} < 0 THEN NULL ELSE {count} END)"
        ))
    }

    /// The characters at positions `start` to `start + length − 1` that exist (ISO 9075):
    /// `DuckDB` counts a start before 1 from the end of the text instead.
    fn substring(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let (text, start, length) = match arguments {
            [text, start] => (text, start, None),
            [text, start, length] => (text, start, Some(length)),
            _ => return Err(self.error(InvariantViolation::MalformedFunctionCall)),
        };
        let text = self.write(text)?;
        let start = self.operand(start)?;
        let first = format!("greatest({start}, 1)");
        let Some(length) = length else {
            return Ok(format!("substring({text}, {first})"));
        };
        let length = self.operand(length)?;
        let count = format!(
            "CAST(least(greatest(CAST({start} AS HUGEINT) + {length} - {first}, 0), \
             length({text})) AS BIGINT)"
        );
        Ok(format!(
            "substring({text}, {first}, CASE WHEN {length} < 0 THEN NULL ELSE {count} END)"
        ))
    }

    /// `replace(s, from, to)`, with `s` unchanged for an empty `from`.
    fn replace(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [text, from, to] = self.exactly(arguments)?;
        let text = self.operand(text)?;
        let from = self.operand(from)?;
        let to = self.operand(to)?;
        Ok(format!(
            "CASE WHEN {text} IS NULL OR {from} IS NULL OR {to} IS NULL THEN NULL \
             WHEN {from} = '' THEN {text} ELSE replace({text}, {from}, {to}) END"
        ))
    }

    /// `split_part(s, delimiter, n)`, the literal `n` written in the SQL (E6-4.11). An empty
    /// delimiter leaves `s` as its only part: `s` for `n = 1`, empty text otherwise.
    fn split_part(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [text, delimiter, number] = self.exactly(arguments)?;
        let number = self.literal_integer(number)?;
        let text = self.operand(text)?;
        let delimiter = self.operand(delimiter)?;
        let only_part = if number == 1 { text.as_str() } else { "''" };
        Ok(format!(
            "CASE WHEN {text} IS NULL OR {delimiter} IS NULL THEN NULL \
             WHEN {delimiter} = '' THEN {only_part} ELSE split_part({text}, {delimiter}, {number}) END"
        ))
    }

    /// `POSITION(a IN s)` as `strpos(s, a)`, 1 when `a` is empty.
    fn position(&mut self, arguments: &[Expression]) -> Result<String, GenerateError> {
        let [needle, text] = self.exactly(arguments)?;
        let needle = self.operand(needle)?;
        let text = self.operand(text)?;
        Ok(format!(
            "CASE WHEN {needle} = '' AND {text} IS NOT NULL THEN 1 ELSE strpos({text}, {needle}) END"
        ))
    }
}
