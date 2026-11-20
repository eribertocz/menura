//! The JSON form of a normalized tree (E0H-3.4): the document every hash of a tree covers.
//!
//! Each node is an object with a `node` member naming its kind; operands and arguments keep
//! their order (E0H-1.4). Names are written normalized (E0D-1.1), so a regular identifier and
//! the delimited identifier of its normalized text have the same form (E0H-1.3). Literals keep
//! their value and their type as written (`1.50` has scale 2); nothing is rewritten.

mod literal;
mod words;

use super::call::AggregateCall;
use super::cast::CastTarget;
use super::name::{Identifier, Reference};
use super::tree::{CaseBranch, Expression};
use crate::json::JsonForm;

impl Expression {
    /// The JSON form of the tree (E0H-3.4).
    ///
    /// ```
    /// use menura_model::expression::{Expression, Identifier, Reference};
    ///
    /// let regular = Expression::Reference(Reference {
    ///     qualifier: None,
    ///     name: Identifier::regular("amount").unwrap(),
    /// });
    /// let delimited = Expression::Reference(Reference {
    ///     qualifier: None,
    ///     name: Identifier::delimited("AMOUNT").unwrap(),
    /// });
    /// assert_eq!(regular.json_form(), delimited.json_form());
    /// ```
    #[must_use]
    pub fn json_form(&self) -> JsonForm {
        match self {
            Self::Literal(value) => literal::literal(value),
            Self::Reference(reference) => node("reference", [("name", reference.json_form())]),
            Self::MetricAggregate(reference) => {
                node("metric_aggregate", [("metric", reference.json_form())])
            }
            Self::UserAttribute(name) => node("user_attribute", [("name", JsonForm::text(name))]),
            Self::CurrentDate => node("current_date", []),
            Self::CurrentTime => node("current_time", []),
            Self::CurrentTimestamp => node("current_timestamp", []),
            Self::Unary { operator, operand } => node(
                "unary",
                [
                    ("operator", JsonForm::text(words::unary(*operator))),
                    ("operand", operand.json_form()),
                ],
            ),
            Self::Binary {
                operator,
                left,
                right,
            } => node(
                "binary",
                [
                    ("operator", JsonForm::text(words::binary(*operator))),
                    ("left", left.json_form()),
                    ("right", right.json_form()),
                ],
            ),
            _ => self.predicate_form().unwrap_or_else(|| self.call_form()),
        }
    }

    /// The form of the predicates and of `CASE`, or `None` for another node.
    fn predicate_form(&self) -> Option<JsonForm> {
        Some(match self {
            Self::IsNull { operand, negated } => node(
                "is_null",
                [
                    ("operand", operand.json_form()),
                    ("negated", flag(*negated)),
                ],
            ),
            Self::IsDistinctFrom {
                left,
                right,
                negated,
            } => node(
                "is_distinct_from",
                [
                    ("left", left.json_form()),
                    ("right", right.json_form()),
                    ("negated", flag(*negated)),
                ],
            ),
            Self::InList {
                operand,
                list,
                negated,
            } => node(
                "in_list",
                [
                    ("operand", operand.json_form()),
                    ("list", trees(list)),
                    ("negated", flag(*negated)),
                ],
            ),
            Self::Between {
                operand,
                low,
                high,
                negated,
            } => node(
                "between",
                [
                    ("operand", operand.json_form()),
                    ("low", low.json_form()),
                    ("high", high.json_form()),
                    ("negated", flag(*negated)),
                ],
            ),
            _ => return self.text_predicate_form(),
        })
    }

    /// The form of `LIKE` and `CASE`, or `None` for another node.
    fn text_predicate_form(&self) -> Option<JsonForm> {
        Some(match self {
            Self::Like {
                operand,
                pattern,
                case,
                negated,
            } => node(
                words::like_case(*case),
                [
                    ("operand", operand.json_form()),
                    ("pattern", pattern.json_form()),
                    ("negated", flag(*negated)),
                ],
            ),
            Self::Case {
                operand,
                branches,
                otherwise,
            } => node(
                "case",
                [
                    ("operand", optional_tree(operand.as_deref())),
                    ("branches", JsonForm::list(branches, CaseBranch::json_form)),
                    ("otherwise", optional_tree(otherwise.as_deref())),
                ],
            ),
            _ => return None,
        })
    }

    /// The form of casts, date functions, scalar functions and aggregates.
    fn call_form(&self) -> JsonForm {
        match self {
            Self::Cast { operand, target } => node(
                "cast",
                [
                    ("operand", operand.json_form()),
                    ("target", cast_target(*target)),
                ],
            ),
            Self::Extract { part, operand } => node(
                "extract",
                [
                    ("part", JsonForm::text(words::extract_part(*part))),
                    ("operand", operand.json_form()),
                ],
            ),
            Self::DateTrunc { unit, operand } => node(
                "date_trunc",
                [
                    ("unit", JsonForm::text(words::date_unit(*unit))),
                    ("operand", operand.json_form()),
                ],
            ),
            Self::DateAdd {
                unit,
                amount,
                operand,
            } => node(
                "date_add",
                [
                    ("unit", JsonForm::text(words::date_unit(*unit))),
                    ("amount", amount.json_form()),
                    ("operand", operand.json_form()),
                ],
            ),
            Self::DateDiff { unit, start, end } => node(
                "date_diff",
                [
                    ("unit", JsonForm::text(words::date_unit(*unit))),
                    ("start", start.json_form()),
                    ("end", end.json_form()),
                ],
            ),
            _ => self.function_form(),
        }
    }

    /// The form of scalar functions and aggregates; every other node has its own form.
    fn function_form(&self) -> JsonForm {
        match self {
            Self::Function {
                function,
                arguments,
            } => node(
                "function",
                [
                    (
                        "function",
                        JsonForm::text(words::scalar_function(*function)),
                    ),
                    ("arguments", trees(arguments)),
                ],
            ),
            Self::Aggregate { call, filter } => node(
                "aggregate",
                [
                    ("call", call.json_form()),
                    ("filter", optional_tree(filter.as_deref())),
                ],
            ),
            _ => JsonForm::Null,
        }
    }
}

impl AggregateCall {
    /// The JSON form of the call (E0H-3.4), without its `FILTER`.
    #[must_use]
    pub fn json_form(&self) -> JsonForm {
        match self {
            Self::CountRows => JsonForm::object([("function", JsonForm::text("count_rows"))]),
            Self::Simple { function, argument } => JsonForm::object([
                (
                    "function",
                    JsonForm::text(words::simple_aggregate(*function)),
                ),
                ("argument", argument.json_form()),
            ]),
            Self::Distinct { function, argument } => JsonForm::object([
                (
                    "function",
                    JsonForm::text(words::distinct_aggregate(*function)),
                ),
                ("distinct", JsonForm::Boolean(true)),
                ("argument", argument.json_form()),
            ]),
            Self::Percentile {
                function,
                fraction,
                ordering,
                direction,
            } => JsonForm::object([
                ("function", JsonForm::text(words::percentile(*function))),
                ("fraction", fraction.json_form()),
                ("ordering", ordering.json_form()),
                ("direction", JsonForm::text(words::direction(*direction))),
            ]),
        }
    }
}

impl CaseBranch {
    /// The JSON form of the branch (E0H-3.4).
    #[must_use]
    pub fn json_form(&self) -> JsonForm {
        JsonForm::object([
            ("when", self.condition.json_form()),
            ("then", self.result.json_form()),
        ])
    }
}

impl Reference {
    /// The JSON form of the reference: `{"qualifier": …, "name": …}` with normalized names
    /// (E0H-1.3); `qualifier` is `null` when none is written.
    #[must_use]
    pub fn json_form(&self) -> JsonForm {
        JsonForm::object([
            (
                "qualifier",
                JsonForm::optional(
                    self.qualifier
                        .as_ref()
                        .map(|qualifier| JsonForm::Text(qualifier.normalized())),
                ),
            ),
            ("name", JsonForm::Text(self.name.normalized())),
        ])
    }
}

impl Identifier {
    /// The normalized name the identifier refers to (E0D-1.1): a regular identifier in upper
    /// case, a delimited one exactly as written without its quotes.
    ///
    /// ```
    /// use menura_model::expression::Identifier;
    /// assert_eq!(Identifier::regular("amount").unwrap().normalized(), "AMOUNT");
    /// assert_eq!(Identifier::delimited("amount").unwrap().normalized(), "amount");
    /// ```
    #[must_use]
    pub fn normalized(&self) -> String {
        if self.is_delimited() {
            self.text().to_owned()
        } else {
            menura_contracts::normalize_unquoted_identifier(self.text())
        }
    }
}

/// A node object: `{"node": kind, …members}`.
fn node<const COUNT: usize>(kind: &str, members: [(&str, JsonForm); COUNT]) -> JsonForm {
    let mut object = JsonForm::object(members);
    if let JsonForm::Object(entries) = &mut object {
        entries.insert("node".to_owned(), JsonForm::text(kind));
    }
    object
}

fn flag(value: bool) -> JsonForm {
    JsonForm::Boolean(value)
}

fn trees(expressions: &[Expression]) -> JsonForm {
    JsonForm::list(expressions, Expression::json_form)
}

fn optional_tree(expression: Option<&Expression>) -> JsonForm {
    JsonForm::optional(expression.map(Expression::json_form))
}

fn cast_target(target: CastTarget) -> JsonForm {
    match target {
        CastTarget::Decimal(decimal) => JsonForm::object([
            ("type", JsonForm::text(words::cast_target(target))),
            (
                "precision",
                JsonForm::Integer(i64::from(decimal.precision())),
            ),
            ("scale", JsonForm::Integer(i64::from(decimal.scale()))),
        ]),
        _ => JsonForm::object([("type", JsonForm::text(words::cast_target(target)))]),
    }
}
