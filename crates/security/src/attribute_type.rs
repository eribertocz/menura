//! The types of security-context attributes (SCY-2.2), and how their text is read
//! (SCY-5.1.2).

use std::fmt;

/// The greatest precision of `decimal(p,s)` (SCY-5.1.2).
const MAXIMUM_PRECISION: u8 = 38;

/// A type an attribute, or an item of a list attribute, can have (SCY-2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarType {
    /// `text`.
    Text,
    /// `integer`.
    Integer,
    /// `decimal(p,s)`, with 1 ≤ p ≤ 38 and 0 ≤ s ≤ p.
    Decimal {
        /// Total digits.
        precision: u8,
        /// Digits after the point.
        scale: u8,
    },
    /// `date`.
    Date,
    /// `boolean`.
    Boolean,
}

/// The type of an attribute (SCY-2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeType {
    /// A single value.
    Scalar(ScalarType),
    /// `list<…>`: a list of values of a type that is not a list.
    List(ScalarType),
}

impl AttributeType {
    /// The type `text` writes, as SCY-2.2 writes it: `text`, `integer`, `decimal(p,s)`,
    /// `date`, `boolean`, or `list<…>` of one of them; `None` for any other text (SCY-5.1.2).
    ///
    /// ```
    /// use semantic_compiler_security::{AttributeType, ScalarType};
    ///
    /// assert_eq!(AttributeType::parse("list<text>"), Some(AttributeType::List(ScalarType::Text)));
    /// assert_eq!(
    ///     AttributeType::parse("decimal(18,2)"),
    ///     Some(AttributeType::Scalar(ScalarType::Decimal { precision: 18, scale: 2 }))
    /// );
    /// assert_eq!(AttributeType::parse("decimal(18)"), None);
    /// ```
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text
            .strip_prefix("list<")
            .and_then(|rest| rest.strip_suffix('>'))
        {
            Some(item) => ScalarType::parse(item).map(Self::List),
            None => ScalarType::parse(text).map(Self::Scalar),
        }
    }

    /// Whether the attribute is a list.
    #[must_use]
    pub fn is_list(self) -> bool {
        matches!(self, Self::List(_))
    }

    /// The type of the attribute, or of each of its items for a list.
    #[must_use]
    pub fn item(self) -> ScalarType {
        match self {
            Self::Scalar(item) | Self::List(item) => item,
        }
    }
}

impl ScalarType {
    /// The scalar type `text` writes, or `None`.
    fn parse(text: &str) -> Option<Self> {
        match text {
            "text" => Some(Self::Text),
            "integer" => Some(Self::Integer),
            "date" => Some(Self::Date),
            "boolean" => Some(Self::Boolean),
            _ => Self::decimal(text),
        }
    }

    /// `decimal(p,s)` with 1 ≤ p ≤ 38 and 0 ≤ s ≤ p, or `None`.
    fn decimal(text: &str) -> Option<Self> {
        let arguments = text.strip_prefix("decimal(")?.strip_suffix(')')?;
        let (precision, scale) = arguments.split_once(',')?;
        let (precision, scale) = (number(precision)?, number(scale)?);
        (1..=MAXIMUM_PRECISION)
            .contains(&precision)
            .then_some(())
            .filter(|()| scale <= precision)
            .map(|()| Self::Decimal { precision, scale })
    }
}

/// A number written in decimal digits without a sign or a leading zero, such as `18`, that
/// fits in a `u8`.
fn number(text: &str) -> Option<u8> {
    let digits = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let canonical = text == "0" || !text.starts_with('0');
    if digits && canonical {
        text.parse().ok()
    } else {
        None
    }
}

impl fmt::Display for ScalarType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => formatter.write_str("text"),
            Self::Integer => formatter.write_str("integer"),
            Self::Decimal { precision, scale } => write!(formatter, "decimal({precision},{scale})"),
            Self::Date => formatter.write_str("date"),
            Self::Boolean => formatter.write_str("boolean"),
        }
    }
}

impl fmt::Display for AttributeType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(item) => write!(formatter, "{item}"),
            Self::List(item) => write!(formatter, "list<{item}>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scy_2_2_every_type_is_read() {
        let scalars = [
            ("text", ScalarType::Text),
            ("integer", ScalarType::Integer),
            ("date", ScalarType::Date),
            ("boolean", ScalarType::Boolean),
            (
                "decimal(18,2)",
                ScalarType::Decimal {
                    precision: 18,
                    scale: 2,
                },
            ),
        ];
        for (text, scalar) in scalars {
            assert_eq!(
                AttributeType::parse(text),
                Some(AttributeType::Scalar(scalar))
            );
            let list = format!("list<{text}>");
            assert_eq!(
                AttributeType::parse(&list),
                Some(AttributeType::List(scalar))
            );
            assert_eq!(AttributeType::List(scalar).to_string(), list);
        }
    }

    #[test]
    fn scy_5_1_2_decimal_bounds() {
        let decimal = |text: &str| AttributeType::parse(text);
        assert!(decimal("decimal(1,0)").is_some());
        assert!(decimal("decimal(38,38)").is_some());
        for text in [
            "decimal(0,0)",
            "decimal(39,2)",
            "decimal(5,6)",
            "decimal(18)",
            "decimal",
            "decimal(18, 2)",
            "decimal(018,2)",
            "decimal(-1,0)",
            "decimal(300,2)",
        ] {
            assert_eq!(decimal(text), None, "{text}");
        }
    }

    #[test]
    fn scy_5_1_2_other_texts_are_not_types() {
        for text in [
            "",
            "TEXT",
            "string",
            "list<list<text>>",
            "list<>",
            "list<text",
            "list <text>",
            " text",
        ] {
            assert_eq!(AttributeType::parse(text), None, "{text}");
        }
    }
}
