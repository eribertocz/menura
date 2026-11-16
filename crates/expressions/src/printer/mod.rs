//! Writes a tree as canonical expression text (E0H-1.3).
//!
//! Keywords are upper case, operators have one space on each side, and every operand that is
//! not self-delimiting is put in parentheses, so precedence never has to be inferred. Parsing
//! the printed text gives back the same tree.

mod constants;

use std::fmt::{self, Display, Formatter};

use semantic_compiler_model::expression::{
    AggregateCall, BinaryOperator, CaseBranch, DistinctAggregate, Expression, LikeCase,
    PercentileFunction, ScalarFunction, SortDirection, UnaryOperator,
};

use crate::vocabulary::{
    date_unit_name, extract_part_name, scalar_function_name, simple_aggregate_name,
};
use constants::{Quoted, write_cast_target, write_literal, write_reference};

/// Writes `expression` as canonical text.
///
/// For every tree that [`crate::parse`] returns, parsing the printed text returns the same tree.
///
/// ```
/// use semantic_compiler_expressions::{parse, print};
/// let tree = parse("not (a)is null").unwrap();
/// assert_eq!(print(&tree), "NOT (a IS NULL)");
/// ```
#[must_use]
pub fn print(expression: &Expression) -> String {
    Canonical(expression).to_string()
}

/// An expression displayed as canonical text.
struct Canonical<'tree>(&'tree Expression);

impl Display for Canonical<'_> {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        write_expression(output, self.0)
    }
}

/// An operand: in parentheses unless its text is self-delimiting.
struct Operand<'tree>(&'tree Expression);

impl Display for Operand<'_> {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        let self_delimiting = !matches!(
            self.0,
            Expression::Unary { .. }
                | Expression::Binary { .. }
                | Expression::IsNull { .. }
                | Expression::IsDistinctFrom { .. }
                | Expression::InList { .. }
                | Expression::Between { .. }
                | Expression::Like { .. }
        );
        if self_delimiting {
            write_expression(output, self.0)
        } else {
            write!(output, "({})", Canonical(self.0))
        }
    }
}

/// `(a, b, …)`.
struct Arguments<'tree>(&'tree [Expression]);

impl Display for Arguments<'_> {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        output.write_str("(")?;
        for (index, argument) in self.0.iter().enumerate() {
            if index > 0 {
                output.write_str(", ")?;
            }
            write_expression(output, argument)?;
        }
        output.write_str(")")
    }
}

/// `NOT ` when `negated`.
fn not(negated: bool) -> &'static str {
    if negated { "NOT " } else { "" }
}

fn write_expression(output: &mut Formatter<'_>, expression: &Expression) -> fmt::Result {
    match expression {
        Expression::Unary { operator, operand } => {
            let symbol = match operator {
                UnaryOperator::Plus => "+",
                UnaryOperator::Minus => "-",
                UnaryOperator::Not => "NOT ",
            };
            write!(output, "{symbol}{}", Operand(operand))
        }
        Expression::Binary {
            operator,
            left,
            right,
        } => {
            let symbol = binary_operator_text(*operator);
            write!(output, "{} {symbol} {}", Operand(left), Operand(right))
        }
        Expression::Case {
            operand,
            branches,
            otherwise,
        } => write_case(output, operand.as_deref(), branches, otherwise.as_deref()),
        Expression::Cast { operand, target } => {
            write!(output, "CAST({} AS ", Canonical(operand))?;
            write_cast_target(output, *target)?;
            output.write_str(")")
        }
        Expression::Function {
            function,
            arguments,
        } => write_function(output, *function, arguments),
        Expression::Aggregate { call, filter } => write_aggregate(output, call, filter.as_deref()),
        Expression::Extract { .. }
        | Expression::DateTrunc { .. }
        | Expression::DateAdd { .. }
        | Expression::DateDiff { .. } => write_date_function(output, expression),
        Expression::IsNull { .. } | Expression::IsDistinctFrom { .. } => {
            write_null_test(output, expression)
        }
        Expression::InList { .. } | Expression::Between { .. } | Expression::Like { .. } => {
            write_match_test(output, expression)
        }
        atom => write_atom(output, atom),
    }
}

/// Expressions without operands.
fn write_atom(output: &mut Formatter<'_>, expression: &Expression) -> fmt::Result {
    match expression {
        Expression::Literal(literal) => write_literal(output, literal),
        Expression::Reference(reference) => write_reference(output, reference),
        Expression::MetricAggregate(reference) => {
            output.write_str("AGG(")?;
            write_reference(output, reference)?;
            output.write_str(")")
        }
        Expression::UserAttribute(name) => write!(output, "USER_ATTRIBUTE({})", Quoted(name, '\'')),
        Expression::CurrentDate => output.write_str("CURRENT_DATE"),
        Expression::CurrentTime => output.write_str("CURRENT_TIME"),
        Expression::CurrentTimestamp => output.write_str("CURRENT_TIMESTAMP"),
        other => write_expression(output, other),
    }
}

/// `IS [NOT] NULL` and `IS [NOT] DISTINCT FROM`.
fn write_null_test(output: &mut Formatter<'_>, expression: &Expression) -> fmt::Result {
    match expression {
        Expression::IsNull { operand, negated } => {
            write!(output, "{} IS {}NULL", Operand(operand), not(*negated))
        }
        Expression::IsDistinctFrom {
            left,
            right,
            negated,
        } => write!(
            output,
            "{} IS {}DISTINCT FROM {}",
            Operand(left),
            not(*negated),
            Operand(right)
        ),
        other => write_expression(output, other),
    }
}

/// `[NOT] IN`, `[NOT] BETWEEN`, `[NOT] LIKE` and `[NOT] ILIKE`.
fn write_match_test(output: &mut Formatter<'_>, expression: &Expression) -> fmt::Result {
    match expression {
        Expression::InList {
            operand,
            list,
            negated,
        } => {
            write!(
                output,
                "{} {}IN {}",
                Operand(operand),
                not(*negated),
                Arguments(list)
            )
        }
        Expression::Between {
            operand,
            low,
            high,
            negated,
        } => write!(
            output,
            "{} {}BETWEEN {} AND {}",
            Operand(operand),
            not(*negated),
            Operand(low),
            Operand(high)
        ),
        Expression::Like {
            operand,
            pattern,
            case,
            negated,
        } => {
            let keyword = match case {
                LikeCase::Sensitive => "LIKE",
                LikeCase::Insensitive => "ILIKE",
            };
            write!(
                output,
                "{} {}{keyword} {}",
                Operand(operand),
                not(*negated),
                Operand(pattern)
            )
        }
        other => write_expression(output, other),
    }
}

/// A plain call; `POSITION` keeps its own form, `POSITION(a IN s)`.
fn write_function(
    output: &mut Formatter<'_>,
    function: ScalarFunction,
    arguments: &[Expression],
) -> fmt::Result {
    if let (ScalarFunction::Position, [needle, haystack]) = (function, arguments) {
        return write!(
            output,
            "POSITION({} IN {})",
            Operand(needle),
            Canonical(haystack)
        );
    }
    write!(
        output,
        "{}{}",
        scalar_function_name(function),
        Arguments(arguments)
    )
}

/// `EXTRACT`, `DATE_TRUNC`, `DATEADD` and `DATEDIFF`, with the part as E0F-10.1 writes it.
fn write_date_function(output: &mut Formatter<'_>, expression: &Expression) -> fmt::Result {
    match expression {
        Expression::Extract { part, operand } => {
            write!(
                output,
                "EXTRACT({} FROM {})",
                extract_part_name(*part),
                Canonical(operand)
            )
        }
        Expression::DateTrunc { unit, operand } => {
            write!(
                output,
                "DATE_TRUNC('{}', {})",
                date_unit_name(*unit),
                Canonical(operand)
            )
        }
        Expression::DateAdd {
            unit,
            amount,
            operand,
        } => write!(
            output,
            "DATEADD({}, {}, {})",
            date_unit_name(*unit),
            Canonical(amount),
            Canonical(operand)
        ),
        Expression::DateDiff { unit, start, end } => write!(
            output,
            "DATEDIFF({}, {}, {})",
            date_unit_name(*unit),
            Canonical(start),
            Canonical(end)
        ),
        other => write_expression(output, other),
    }
}

fn binary_operator_text(operator: BinaryOperator) -> &'static str {
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

fn write_case(
    output: &mut Formatter<'_>,
    operand: Option<&Expression>,
    branches: &[CaseBranch],
    otherwise: Option<&Expression>,
) -> fmt::Result {
    output.write_str("CASE")?;
    if let Some(operand) = operand {
        write!(output, " {}", Canonical(operand))?;
    }
    for branch in branches {
        write!(
            output,
            " WHEN {} THEN {}",
            Canonical(&branch.condition),
            Canonical(&branch.result)
        )?;
    }
    if let Some(otherwise) = otherwise {
        write!(output, " ELSE {}", Canonical(otherwise))?;
    }
    output.write_str(" END")
}

fn write_aggregate(
    output: &mut Formatter<'_>,
    call: &AggregateCall,
    filter: Option<&Expression>,
) -> fmt::Result {
    match call {
        AggregateCall::CountRows => output.write_str("COUNT(*)")?,
        AggregateCall::Simple { function, argument } => {
            write!(
                output,
                "{}({})",
                simple_aggregate_name(*function),
                Canonical(argument)
            )?;
        }
        AggregateCall::Distinct { function, argument } => {
            let name = match function {
                DistinctAggregate::Sum => "SUM",
                DistinctAggregate::Count => "COUNT",
            };
            write!(output, "{name}(DISTINCT {})", Canonical(argument))?;
        }
        AggregateCall::Percentile {
            function,
            fraction,
            ordering,
            direction,
        } => {
            let name = match function {
                PercentileFunction::Continuous => "PERCENTILE_CONT",
                PercentileFunction::Discrete => "PERCENTILE_DISC",
            };
            let direction = match direction {
                SortDirection::Ascending => "",
                SortDirection::Descending => " DESC",
            };
            write!(
                output,
                "{name}({}) WITHIN GROUP (ORDER BY {}{direction})",
                Canonical(fraction),
                Canonical(ordering)
            )?;
        }
    }
    if let Some(filter) = filter {
        write!(output, " FILTER (WHERE {})", Canonical(filter))?;
    }
    Ok(())
}
