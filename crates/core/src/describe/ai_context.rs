//! The `ai_context` member of a described element, with its provenance (DSC-3.6).

use menura_contracts::Sha256Hash;
use menura_contracts::types::descriptions::{
    AiContext, AiContextSource, AiContextTrust, AttributedAiContext,
};
use menura_model::internal_model::{Descriptions, SourceLocation};

use crate::json_form::json_value;

/// The `ai_context` of the element written at `location` with `descriptions`, in the model
/// of `model_hash`: present only when the element declares one (DSC-3.6, DSC-3.11).
pub(super) fn ai_context(
    descriptions: &Descriptions,
    location: &SourceLocation,
    model_hash: &Sha256Hash,
) -> Option<AiContext> {
    let declared = descriptions.ai_context.as_ref()?;
    Some(AiContext::Attributed(AttributedAiContext {
        value: json_value(declared),
        source: AiContextSource {
            pointer: format!("{}/ai_context", location.pointer),
            model_hash: model_hash.clone(),
        },
        trust: AiContextTrust::UntrustedModelContent,
    }))
}

#[cfg(test)]
mod tests {
    use menura_model::json::JsonForm;
    use serde_json::json;

    use super::json_value;

    #[test]
    fn dsc_3_6_the_value_is_the_declared_json_value() {
        let form = JsonForm::object([
            ("rank", JsonForm::Integer(1)),
            ("weight", JsonForm::Number("1.5e0".to_owned())),
            (
                "tags",
                JsonForm::List(vec![JsonForm::text("a"), JsonForm::Null]),
            ),
            ("on", JsonForm::Boolean(true)),
        ]);
        assert_eq!(
            json_value(&form),
            json!({"rank": 1, "weight": 1.5, "tags": ["a", null], "on": true})
        );
    }
}
