//! Types operators, `CASE` and the comparisons of E0F-2.2.

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::{BinaryOperator, CaseBranch, Expression, UnaryOperator};

use super::arithmetic::{Arithmetic, arithmetic, widened};
use super::conversion::common_type;
use super::stop::{Outcome, Stop, incompatible, untyped_null};
use super::typer::{ExpressionTyper, any_nullable, expect_numeric, expect_text};
use super::value_type::{ExpressionType, Typed, describe};

impl ExpressionTyper<'_> {
    /// The type of an operator expression or a `CASE`.
    pub(super) fn operator(&self, expression: &Expression) -> Outcome<Typed> {
        match expression {
            Expression::Unary { operator, operand } => self.unary(*operator, operand),
            Expression::Binary {
                operator,
                left,
                right,
            } => self.binary(*operator, left, right),
            Expression::IsNull { operand, .. } => {
                self.type_of(operand)?;
                Ok(Typed::value(LogicalType::Boolean, false))
            }
            Expression::IsDistinctFrom { left, right, .. } => {
                let operands = self.types_of_all(&[left, right])?;
                comparable(&operands)?;
                Ok(Typed::value(LogicalType::Boolean, false))
            }
            Expression::InList { operand, list, .. } => {
                let operands = self.types_of_all(&[operand])?;
                let operands = [operands, self.types_of(list)?].concat();
                comparison(&operands)
            }
            Expression::Between {
                operand, low, high, ..
            } => comparison(&self.types_of_all(&[operand, low, high])?),
            Expression::Like {
                operand, pattern, ..
            } => {
                let operands = self.types_of_all(&[operand, pattern])?;
                for typed in &operands {
                    expect_text(typed, "an operand of LIKE and ILIKE")?;
                }
                Ok(Typed::value(LogicalType::Boolean, any_nullable(&operands)))
            }
            Expression::Case {
                operand,
                branches,
                otherwise,
            } => self.case(operand.as_deref(), branches, otherwise.as_deref()),
            _ => Err(Stop::ReportedElsewhere),
        }
    }

    /// The types of `expressions`, in order.
    pub(super) fn types_of_all(&self, expressions: &[&Expression]) -> Outcome<Vec<Typed>> {
        expressions
            .iter()
            .map(|expression| self.type_of(expression))
            .collect()
    }

    fn unary(&self, operator: UnaryOperator, operand: &Expression) -> Outcome<Typed> {
        if operator == UnaryOperator::Not {
            let nullable = self.condition(operand, "the operand of NOT")?;
            return Ok(Typed::value(LogicalType::Boolean, nullable));
        }
        let typed = self.type_of(operand)?;
        let role = "the operand of a sign";
        let Some(kind) = expect_numeric(&typed, role)? else {
            return untyped_null(role);
        };
        let value_type = typed.logical().cloned().ok_or(Stop::ReportedElsewhere)?;
        let value_type = match operator {
            UnaryOperator::Minus => widened(kind, &value_type),
            UnaryOperator::Plus | UnaryOperator::Not => value_type,
        };
        Ok(Typed::value(value_type, typed.nullable))
    }

    fn binary(
        &self,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
    ) -> Outcome<Typed> {
        let operation = match operator {
            BinaryOperator::And | BinaryOperator::Or => {
                let left = self.condition(left, "an operand of AND and OR")?;
                let right = self.condition(right, "an operand of AND and OR")?;
                return Ok(Typed::value(LogicalType::Boolean, left || right));
            }
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Modulo => {
                Arithmetic::AddSubtractModulo
            }
            BinaryOperator::Multiply => Arithmetic::Multiply,
            BinaryOperator::Divide => Arithmetic::Divide,
            BinaryOperator::Concatenate => {
                let operands = self.types_of_all(&[left, right])?;
                for typed in &operands {
                    expect_text(typed, "an operand of ||")?;
                }
                return Ok(Typed::value(LogicalType::Text, any_nullable(&operands)));
            }
            _ => return comparison(&self.types_of_all(&[left, right])?),
        };
        let operands = self.types_of_all(&[left, right])?;
        let value_type = arithmetic_type(operation, &operands)?;
        // Division and modulo by zero yield NULL (E0F-6.1, E0F-10.3).
        let by_zero = matches!(operator, BinaryOperator::Divide | BinaryOperator::Modulo);
        Ok(Typed::value(value_type, by_zero || any_nullable(&operands)))
    }

    fn case(
        &self,
        operand: Option<&Expression>,
        branches: &[CaseBranch],
        otherwise: Option<&Expression>,
    ) -> Outcome<Typed> {
        let compared = operand.map(|operand| self.type_of(operand)).transpose()?;
        let mut results = Vec::new();
        for branch in branches {
            match &compared {
                Some(compared) => {
                    let value = self.type_of(&branch.condition)?;
                    comparable(&[compared.clone(), value])?;
                }
                None => {
                    self.condition(&branch.condition, "a WHEN condition")?;
                }
            }
            results.push(self.type_of(&branch.result)?);
        }
        if let Some(otherwise) = otherwise {
            results.push(self.type_of(otherwise)?);
        }
        let value_type = case_rule(&results, "the results of CASE")?;
        let nullable = otherwise.is_none() || any_nullable(&results);
        Ok(Typed {
            value_type,
            nullable,
        })
    }
}

/// The type of an arithmetic operation over `operands` (E0F-5a).
pub(super) fn arithmetic_type(operation: Arithmetic, operands: &[Typed]) -> Outcome<LogicalType> {
    let [left, right] = operands else {
        return Err(Stop::ReportedElsewhere);
    };
    let role = "an operand of arithmetic";
    let left = expect_numeric(left, role)?;
    let right = expect_numeric(right, role)?;
    // The literal `NULL` takes the type of the other operand (E0F-3.2).
    match (left.or(right), right.or(left)) {
        (Some(left), Some(right)) => arithmetic(operation, left, right),
        _ => untyped_null(role),
    }
}

/// The CASE rule over `typed` (E0F-5a): `E0F-03` when no type receives every other, or when
/// every operand is the literal `NULL` (E0F-3.2).
pub(super) fn case_rule(operands: &[Typed], role: &str) -> Outcome<ExpressionType> {
    let types: Vec<&ExpressionType> = operands.iter().map(|typed| &typed.value_type).collect();
    match common_type(&types) {
        Some(ExpressionType::Null) => untyped_null(role),
        Some(found) => Ok(found),
        None => incompatible(no_common_type(role, &types)),
    }
}

/// Checks the operands of a comparison, `[NOT] IN`, `[NOT] BETWEEN`, `IS [NOT] DISTINCT FROM`,
/// or a `WHEN` value of a simple `CASE` with its operand (E0F-3.1): numeric operands of any
/// kinds are accepted, and any others need one type that receives every other implicitly (the
/// CASE rule).
pub(super) fn comparable(operands: &[Typed]) -> Outcome<()> {
    if all_numeric(operands) {
        return Ok(());
    }
    let types: Vec<&ExpressionType> = operands.iter().map(|typed| &typed.value_type).collect();
    match common_type(&types) {
        Some(_) => Ok(()),
        None => incompatible(no_common_type("the compared values", &types)),
    }
}

/// Whether every operand is numeric or the literal `NULL`.
fn all_numeric(operands: &[Typed]) -> bool {
    operands
        .iter()
        .all(|typed| expect_numeric(typed, "").is_ok())
}

fn no_common_type(role: &str, types: &[&ExpressionType]) -> String {
    let names: Vec<String> = types
        .iter()
        .map(|value_type| describe(value_type))
        .collect();
    format!(
        "{role} have no common type without CAST: {}",
        names.join(", ")
    )
}

/// A comparison of `operands`: boolean, NULL when any operand is.
fn comparison(operands: &[Typed]) -> Outcome<Typed> {
    comparable(operands)?;
    Ok(Typed::value(LogicalType::Boolean, any_nullable(operands)))
}
