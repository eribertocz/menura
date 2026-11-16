//! Canonical text of literals, identifiers and `CAST` target types.

use std::fmt::{self, Display, Formatter};

use semantic_compiler_model::expression::{
    ApproximateNumber, CastTarget, DateValue, ExactNumber, Identifier, Literal, Reference,
    TimestampValue,
};

use crate::vocabulary::simple_cast_target_name;

/// Text between quote characters, with each quote inside doubled.
pub(super) struct Quoted<'text>(pub(super) &'text str, pub(super) char);

impl Display for Quoted<'_> {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        let Self(text, quote) = *self;
        write!(output, "{quote}")?;
        for character in text.chars() {
            if character == quote {
                write!(output, "{quote}")?;
            }
            write!(output, "{character}")?;
        }
        write!(output, "{quote}")
    }
}

pub(super) fn write_reference(output: &mut Formatter<'_>, reference: &Reference) -> fmt::Result {
    if let Some(qualifier) = &reference.qualifier {
        write_identifier(output, qualifier)?;
        output.write_str(".")?;
    }
    write_identifier(output, &reference.name)
}

fn write_identifier(output: &mut Formatter<'_>, identifier: &Identifier) -> fmt::Result {
    if identifier.is_delimited() {
        write!(output, "{}", Quoted(identifier.text(), '"'))
    } else {
        output.write_str(identifier.text())
    }
}

pub(super) fn write_cast_target(output: &mut Formatter<'_>, target: CastTarget) -> fmt::Result {
    match target {
        CastTarget::Decimal(decimal) => {
            write!(
                output,
                "DECIMAL({}, {})",
                decimal.precision(),
                decimal.scale()
            )
        }
        CastTarget::DoublePrecision => output.write_str("DOUBLE PRECISION"),
        other => output.write_str(simple_cast_target_name(other).unwrap_or_default()),
    }
}

pub(super) fn write_literal(output: &mut Formatter<'_>, literal: &Literal) -> fmt::Result {
    match literal {
        Literal::Null => output.write_str("NULL"),
        Literal::Boolean(true) => output.write_str("TRUE"),
        Literal::Boolean(false) => output.write_str("FALSE"),
        Literal::Text(text) => write!(output, "{}", Quoted(text, '\'')),
        Literal::Exact(number) => write_exact(output, number),
        Literal::Approximate(ApproximateNumber { mantissa, exponent }) => {
            write_exact(output, mantissa)?;
            write!(output, "E{exponent}")
        }
        Literal::Date(date) => {
            output.write_str("DATE '")?;
            write_date(output, *date)?;
            output.write_str("'")
        }
        Literal::Timestamp(timestamp) => write_timestamp(output, timestamp),
    }
}

fn write_exact(output: &mut Formatter<'_>, number: &ExactNumber) -> fmt::Result {
    output.write_str(number.integer_digits())?;
    if number.fraction_digits().is_empty() {
        return Ok(());
    }
    write!(output, ".{}", number.fraction_digits())
}

fn write_date(output: &mut Formatter<'_>, date: DateValue) -> fmt::Result {
    write!(
        output,
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month(),
        date.day()
    )
}

fn write_timestamp(output: &mut Formatter<'_>, timestamp: &TimestampValue) -> fmt::Result {
    let time = &timestamp.time;
    output.write_str("TIMESTAMP '")?;
    write_date(output, timestamp.date)?;
    write!(
        output,
        " {:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second()
    )?;
    if !time.fraction_digits().is_empty() {
        write!(output, ".{}", time.fraction_digits())?;
    }
    output.write_str("'")
}
