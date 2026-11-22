//! How lineage writes model elements (PLN-4.4, PLN-4.7).

use std::collections::BTreeSet;

use menura_model::internal_model::{FieldPath, NormalizedName};

/// The name of a model element as lineage writes it: its normalized name (E0H-1.1).
pub(super) fn written_name(name: &NormalizedName) -> &str {
    name.as_str()
}

/// The `model_inputs` of a column that reads `fields` and `metrics` and whose fields are
/// brought by `relationships`: `field:<dataset>.<field>`, then `metric:<name>`, then
/// `relationship:<name>`, each kind in CON-5 order of the normalized names (PLN-4.4).
pub(super) fn model_inputs(
    fields: &BTreeSet<FieldPath>,
    metrics: &BTreeSet<NormalizedName>,
    relationships: &BTreeSet<NormalizedName>,
) -> Vec<String> {
    let fields = fields.iter().map(|path| {
        format!(
            "field:{}.{}",
            written_name(&path.dataset),
            written_name(&path.field)
        )
    });
    let metrics = metrics
        .iter()
        .map(|name| format!("metric:{}", written_name(name)));
    let relationships = relationships
        .iter()
        .map(|name| format!("relationship:{}", written_name(name)));
    fields.chain(metrics).chain(relationships).collect()
}
