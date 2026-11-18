//! Reading a field reference `dataset.field` of an additivity declaration (ADD-5.1.3), and of
//! a context modifier, which reads its references the same way (CTX-5.1.2).

use semantic_compiler_model::expression::{Expression, Reference};

use super::model::FieldReference;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::names::Name;

/// The qualified reference `text`, or `None` when it has any other form (ADD-5.1.3).
///
/// The text is read by the expression parser, the only reader of the identifier rules of
/// E0F-9.3 (S10): each part is a regular or a quoted identifier, compared as E0D-1.1. A text
/// that does not parse, or that parses as anything but a qualified reference, has another form.
pub(crate) fn field_reference(text: &str) -> Option<FieldReference> {
    let Ok(Expression::Reference(Reference {
        qualifier: Some(dataset),
        name,
    })) = parse_element_expression(text, ExpressionPlace::Extension)
    else {
        return None;
    };
    Some(FieldReference {
        written: text.to_owned(),
        dataset: Name::of(&dataset),
        field: Name::of(&name),
    })
}

#[cfg(test)]
mod tests {
    use super::field_reference;
    use crate::names::Name;

    #[test]
    fn add_5_1_3_a_reference_is_dataset_dot_field() {
        let reference = field_reference("debt.period").expect("a qualified reference");
        assert_eq!(reference.dataset, Name::regular("debt"));
        assert_eq!(reference.field, Name::regular("period"));
        assert_eq!(reference.written, "debt.period");
    }

    #[test]
    fn add_5_1_3_each_part_is_regular_or_quoted() {
        let quoted = field_reference(r#""Debt"."Period""#).expect("a qualified reference");
        assert_eq!(quoted.dataset, Name::declared(r#""Debt""#));
        assert_ne!(quoted.dataset, Name::regular("debt"));
        let upper = field_reference(r#""DEBT".period"#).expect("a qualified reference");
        let lower = field_reference("debt.PERIOD").expect("a qualified reference");
        assert!(upper.same_field(&lower));
    }

    #[test]
    fn add_5_1_3_any_other_form_is_not_a_reference() {
        for text in [
            "period",
            "debt.",
            "",
            "debt.period + 1",
            "SUM(debt.period)",
            "'debt.period'",
            "a.b.c",
        ] {
            assert!(field_reference(text).is_none(), "{text}");
        }
    }
}
