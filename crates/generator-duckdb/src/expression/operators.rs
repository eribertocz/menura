//! Operators, `CASE` and `CAST`.

use menura_model::expression::{BinaryOperator, CaseBranch, CastTarget, Expression, UnaryOperator};

use super::ExpressionWriter;
use crate::error::GenerateError;

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
            BinaryOperator::Modulo => self.modulo(expression, left, right, self.zero_divisor),
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
        let denominator = format!("CAST({} AS DOUBLE)", self.write(denominator)?);
        let numerator = format!("CAST({} AS DOUBLE)", self.write(numerator)?);
        let zero = self.zero_divisor.sql();
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
        let left = self.operand(left)?;
        let right = self.operand(right)?;
        let test = if negated {
            "IS NOT DISTINCT FROM"
        } else {
            "IS DISTINCT FROM"
        };
        Ok(format!("{left} {test} {right}"))
    }

    /// `x [NOT] IN (a, b)`, or `x [NOT] IN (SELECT unnest($n))` for a list attribute (E6-5.3).
    pub(super) fn in_list(
        &mut self,
        operand: &Expression,
        list: &[Expression],
        negated: bool,
    ) -> Result<String, GenerateError> {
        let operand = self.operand(operand)?;
        let keyword = if negated { "NOT IN" } else { "IN" };
        let candidates = match list {
            [Expression::UserAttribute(name)] if self.is_list_attribute(name)? => {
                format!("SELECT unnest({})", self.list_attribute(name)?)
            }
            _ => self.written_list(list)?.join(", "),
        };
        Ok(format!("{operand} {keyword} ({candidates})"))
    }

    pub(super) fn between(
        &mut self,
        operand: &Expression,
        low: &Expression,
        high: &Expression,
        negated: bool,
    ) -> Result<String, GenerateError> {
        let operand_type = self.known_type(operand);
        let operand = self.operand(operand)?;
        let low = self.comparand(low, operand_type.as_ref())?;
        let high = self.comparand(high, operand_type.as_ref())?;
        let keyword = if negated { "NOT BETWEEN" } else { "BETWEEN" };
        Ok(format!("{operand} {keyword} {low} AND {high}"))
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
        let operand = self.write(operand)?;
        Ok(format!("CAST({operand} AS {})", cast_target_sql(target)))
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

fn cast_target_sql(target: CastTarget) -> String {
    match target {
        CastTarget::Text => "VARCHAR".to_owned(),
        CastTarget::Integer => "INTEGER".to_owned(),
        CastTarget::BigInteger => "BIGINT".to_owned(),
        CastTarget::Decimal(decimal) => {
            format!("DECIMAL({},{})", decimal.precision(), decimal.scale())
        }
        CastTarget::Double => "DOUBLE".to_owned(),
        CastTarget::Boolean => "BOOLEAN".to_owned(),
        CastTarget::Date => "DATE".to_owned(),
        CastTarget::Time => "TIME".to_owned(),
        CastTarget::Timestamp => "TIMESTAMP".to_owned(),
    }
}
