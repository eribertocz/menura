//! Operators, `CASE` and `CAST`.

use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};
use menura_model::expression::{BinaryOperator, CaseBranch, CastTarget, Expression, UnaryOperator};

use super::ExpressionWriter;
use crate::error::GenerateError;
use crate::null_safe::not_distinct_by;
use crate::ordering::{Comparison, compared};

impl ExpressionWriter<'_, '_> {
    pub(super) fn unary(
        &mut self,
        expression: &Expression,
        operator: UnaryOperator,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        if operator == UnaryOperator::Minus {
            return self.negation(expression, operand);
        }
        let operand = self.operand(operand)?;
        Ok(match operator {
            UnaryOperator::Plus => format!("+{operand}"),
            UnaryOperator::Minus => format!("-{operand}"),
            UnaryOperator::Not => format!("NOT {operand}"),
        })
    }

    pub(super) fn binary(
        &mut self,
        expression: &Expression,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
    ) -> Result<String, GenerateError> {
        match operator {
            BinaryOperator::Divide => self.division(left, right),
            BinaryOperator::Modulo => self.modulo(expression, left, right),
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply => {
                self.arithmetic(expression, operator, left, right)
            }
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessOrEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterOrEqual => self.comparison(operator, left, right),
            BinaryOperator::Concatenate | BinaryOperator::And | BinaryOperator::Or => {
                let left = self.operand(left)?;
                let right = self.operand(right)?;
                Ok(format!("{left} {} {right}", operator_sql(operator)))
            }
        }
    }

    /// `n / d` as `CASE WHEN d = 0 THEN NULL ELSE n / d END` (E6-4.6), or with the error
    /// `RDER-01` in place of `NULL` inside a derived metric with `on_division_by_zero: error`
    /// (E6-4.6a). Both operands are cast to the 64-bit float of E0F-5a, so neither
    /// `integer_division` nor `inf` can change the result (E6-4.10), and a NULL denominator
    /// gives NULL. The denominator is written first, where it first appears.
    fn division(
        &mut self,
        numerator: &Expression,
        denominator: &Expression,
    ) -> Result<String, GenerateError> {
        let denominator = format!("toFloat64({})", self.write(denominator)?);
        let numerator = format!("toFloat64({})", self.write(numerator)?);
        let zero = self.zero_divisor.sql(&format!("{denominator} = 0"));
        Ok(format!(
            "CASE WHEN {denominator} = 0 THEN {zero} ELSE {numerator} / {denominator} END"
        ))
    }

    pub(super) fn is_null(
        &mut self,
        operand: &Expression,
        negated: bool,
    ) -> Result<String, GenerateError> {
        let operand = self.operand(operand)?;
        let test = if negated { "IS NOT NULL" } else { "IS NULL" };
        Ok(format!("{operand} {test}"))
    }

    pub(super) fn is_distinct_from(
        &mut self,
        left: &Expression,
        right: &Expression,
        negated: bool,
    ) -> Result<String, GenerateError> {
        let comparison = Comparison::of(
            self.known_type(left).as_ref(),
            self.known_type(right).as_ref(),
        );
        let left = self.operand(left)?;
        let right = self.operand(right)?;
        let same = not_distinct_by(&compared(comparison, &left, "=", &right), &left, &right);
        if negated {
            Ok(same)
        } else {
            Ok(format!("NOT {same}"))
        }
    }

    /// `x [NOT] IN (a, b)` with standard NULL semantics (E6-7.7): `x = a OR x = b`, NULL when
    /// `x` is NULL or when no item matches and one is NULL, and its negation; `ClickHouse`'s
    /// own `IN` returns false for NULL instead, and needs constant items. A list attribute,
    /// bound as an array (E6-7.3), holds no NULL item: `x` is in it when `has` finds it, and the
    /// result is NULL when `x` is NULL.
    pub(super) fn in_list(
        &mut self,
        operand: &Expression,
        list: &[Expression],
        negated: bool,
    ) -> Result<String, GenerateError> {
        let comparison = Comparison::of(self.known_type(operand).as_ref(), None);
        let operand = self.operand(operand)?;
        let membership = match list {
            [Expression::UserAttribute(name)] if self.is_list_attribute(name)? => {
                let items = self.list_attribute(name)?;
                format!(
                    "CASE WHEN {operand} IS NULL THEN NULL \
                     WHEN has({items}, assumeNotNull({operand})) THEN TRUE \
                     WHEN has({items}, NULL) THEN NULL ELSE FALSE END"
                )
            }
            _ => {
                let equalities: Vec<String> = self
                    .written_list(list)?
                    .iter()
                    .map(|item| compared(comparison, &operand, "=", item))
                    .collect();
                equalities.join(" OR ")
            }
        };
        if negated {
            Ok(format!("NOT ({membership})"))
        } else {
            Ok(membership)
        }
    }

    pub(super) fn between(
        &mut self,
        operand: &Expression,
        low: &Expression,
        high: &Expression,
        negated: bool,
    ) -> Result<String, GenerateError> {
        let operand_type = self.known_type(operand);
        let comparison = Comparison::of(operand_type.as_ref(), None);
        let operand = self.operand(operand)?;
        let low = self.comparand(low, operand_type.as_ref())?;
        let high = self.comparand(high, operand_type.as_ref())?;
        if comparison == Comparison::Plain {
            let keyword = if negated { "NOT BETWEEN" } else { "BETWEEN" };
            return Ok(format!("{operand} {keyword} {low} AND {high}"));
        }
        let within = format!(
            "{} AND {}",
            compared(comparison, &operand, ">=", &low),
            compared(comparison, &operand, "<=", &high)
        );
        Ok(if negated {
            format!("NOT ({within})")
        } else {
            within
        })
    }

    pub(super) fn case(
        &mut self,
        operand: Option<&Expression>,
        branches: &[CaseBranch],
        otherwise: Option<&Expression>,
    ) -> Result<String, GenerateError> {
        let mut parts = vec!["CASE".to_owned()];
        if let Some(operand) = operand {
            parts.push(self.operand(operand)?);
        }
        for branch in branches {
            let condition = self.operand(&branch.condition)?;
            let result = self.operand(&branch.result)?;
            parts.push(format!("WHEN {condition} THEN {result}"));
        }
        if let Some(otherwise) = otherwise {
            parts.push(format!("ELSE {}", self.operand(otherwise)?));
        }
        parts.push("END".to_owned());
        Ok(parts.join(" "))
    }

    pub(super) fn cast(
        &mut self,
        operand: &Expression,
        target: CastTarget,
    ) -> Result<String, GenerateError> {
        let source = self.known_type(operand);
        let operand = self.write(operand)?;
        let target_type = cast_target_type(target);
        let rounded = rounded_for(source.as_ref(), &target_type, &operand);
        self.cast_value(&rounded, &target_type)
    }

    fn written_list(&mut self, list: &[Expression]) -> Result<Vec<String>, GenerateError> {
        list.iter().map(|item| self.operand(item)).collect()
    }
}

pub(super) fn operator_sql(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Modulo => "%",
        BinaryOperator::Concatenate => "||",
        BinaryOperator::Equal => "=",
        BinaryOperator::NotEqual => "<>",
        BinaryOperator::Less => "<",
        BinaryOperator::LessOrEqual => "<=",
        BinaryOperator::Greater => ">",
        BinaryOperator::GreaterOrEqual => ">=",
        BinaryOperator::And => "AND",
        BinaryOperator::Or => "OR",
    }
}

/// The logical type of a `CAST` target (E0F-8).
fn cast_target_type(target: CastTarget) -> LogicalType {
    match target {
        CastTarget::Text => LogicalType::Text,
        CastTarget::Integer => LogicalType::Integer {
            bits: IntegerBits::ThirtyTwo,
            signed: true,
        },
        CastTarget::BigInteger => LogicalType::Integer {
            bits: IntegerBits::SixtyFour,
            signed: true,
        },
        CastTarget::Decimal(decimal) => LogicalType::Decimal {
            precision: decimal.precision(),
            scale: decimal.scale(),
        },
        CastTarget::Double => LogicalType::Float {
            bits: FloatBits::SixtyFour,
        },
        CastTarget::Boolean => LogicalType::Boolean,
        CastTarget::Date => LogicalType::Date,
        CastTarget::Time => LogicalType::Time,
        CastTarget::Timestamp => LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: false,
        },
    }
}

/// `operand` rounded half away from zero to the scale of `target` when a cast from `source`
/// drops digits: `ClickHouse` truncates them, where E0F-8 rounds (E6-7.9). A decimal is
/// rounded with `round`, which rounds a `Decimal` half away from zero; a float with
/// `sign(x) * floor(abs(x) * 10^s + 0.5) / 10^s`, since `round` rounds a float half to even.
fn rounded_for(source: Option<&LogicalType>, target: &LogicalType, operand: &str) -> String {
    let scale = match target {
        LogicalType::Integer { .. } => 0,
        LogicalType::Decimal { scale, .. } => *scale,
        _ => return operand.to_owned(),
    };
    match source {
        Some(LogicalType::Decimal {
            scale: source_scale,
            ..
        }) if *source_scale > scale => format!("round({operand}, {scale})"),
        Some(LogicalType::Float { .. }) if scale == 0 => {
            format!("sign({operand}) * floor(abs({operand}) + 0.5)")
        }
        Some(LogicalType::Float { .. }) => {
            format!("sign({operand}) * floor(abs({operand}) * 1e{scale} + 0.5) / 1e{scale}")
        }
        _ => operand.to_owned(),
    }
}
