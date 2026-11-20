//! Where a list attribute may appear in a row-policy condition (SCY-4.1).

use menura_model::expression::Expression;

use crate::walk::children;

/// The names of the list attributes that `condition` uses anywhere but as the single item of
/// an `IN` or `NOT IN` list, in written order, each once (SCY-4.1, `E0C-48`).
///
/// `is_list` says whether the attribute of a name is declared as a list; an undeclared
/// attribute is never a list here.
///
/// ```
/// use menura_model::expression::{Expression, Identifier, Reference};
/// use menura_security::misplaced_list_attributes;
///
/// let regions = || Expression::UserAttribute("regions".to_owned());
/// let region = || {
///     Expression::Reference(Reference { qualifier: None, name: Identifier::regular("region").unwrap() })
/// };
/// let is_list = |name: &str| name == "regions";
/// let allowed = Expression::InList { operand: Box::new(region()), list: vec![regions()], negated: false };
/// assert!(misplaced_list_attributes(&allowed, is_list).is_empty());
/// let misplaced = Expression::InList { operand: Box::new(regions()), list: vec![region()], negated: true };
/// assert_eq!(misplaced_list_attributes(&misplaced, is_list), ["regions"]);
/// ```
#[must_use]
pub fn misplaced_list_attributes(
    condition: &Expression,
    is_list: impl Fn(&str) -> bool,
) -> Vec<&str> {
    let mut misplaced = Vec::new();
    visit(condition, false, &is_list, &mut misplaced);
    misplaced
}

/// Records the misplaced list attributes of `expression`, where `allowed` says whether the
/// expression is the single item of an `IN` list.
fn visit<'condition>(
    expression: &'condition Expression,
    allowed: bool,
    is_list: &impl Fn(&str) -> bool,
    misplaced: &mut Vec<&'condition str>,
) {
    match expression {
        Expression::UserAttribute(name) => {
            if !allowed && is_list(name) && !misplaced.contains(&name.as_str()) {
                misplaced.push(name);
            }
        }
        Expression::InList { operand, list, .. } => {
            visit(operand, false, is_list, misplaced);
            let single = list.len() == 1;
            for item in list {
                visit(item, single, is_list, misplaced);
            }
        }
        _ => {
            for child in children(expression) {
                visit(child, false, is_list, misplaced);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use menura_model::expression::{BinaryOperator, Expression, Identifier, Reference};

    use super::misplaced_list_attributes;

    fn attribute(name: &str) -> Expression {
        Expression::UserAttribute(name.to_owned())
    }

    fn field(name: &str) -> Expression {
        Expression::Reference(Reference {
            qualifier: None,
            name: Identifier::regular(name).expect("a regular identifier"),
        })
    }

    fn in_list(operand: Expression, list: Vec<Expression>, negated: bool) -> Expression {
        Expression::InList {
            operand: Box::new(operand),
            list,
            negated,
        }
    }

    fn misplaced(condition: &Expression) -> Vec<&str> {
        misplaced_list_attributes(condition, |name| name.starts_with("list_"))
    }

    #[test]
    fn scy_4_1_the_single_item_of_in_and_not_in_is_allowed() {
        for negated in [false, true] {
            let condition = in_list(field("region"), vec![attribute("list_regions")], negated);
            assert!(misplaced(&condition).is_empty());
        }
    }

    #[test]
    fn scy_4_1_a_list_attribute_anywhere_else_is_misplaced() {
        let equality = Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(field("region")),
            right: Box::new(attribute("list_regions")),
        };
        assert_eq!(misplaced(&equality), ["list_regions"]);
        let two_items = in_list(
            field("region"),
            vec![attribute("list_regions"), attribute("list_regions")],
            false,
        );
        assert_eq!(misplaced(&two_items), ["list_regions"]);
        let operand = in_list(attribute("list_a"), vec![attribute("list_b")], false);
        assert_eq!(misplaced(&operand), ["list_a"]);
        let nested = in_list(
            field("region"),
            vec![Expression::IsNull {
                operand: Box::new(attribute("list_regions")),
                negated: false,
            }],
            false,
        );
        assert_eq!(misplaced(&nested), ["list_regions"]);
    }

    #[test]
    fn scy_4_1_a_scalar_attribute_is_never_misplaced() {
        let equality = Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(field("area_id")),
            right: Box::new(attribute("area_id")),
        };
        assert!(misplaced(&equality).is_empty());
    }
}
