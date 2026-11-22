//! The public function `diff` (DEP-4): which elements two model-dependency documents add,
//! remove or change.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use menura_contracts::types::compiled_plan::{CompileErrorContract, CompileErrorDocument};
use menura_contracts::types::dependencies::{
    ChangeKind, ModelChange, ModelDependenciesDocument, ModelDiffContract, ModelDiffDocument,
};
use menura_contracts::{ContractKind, Sha256Hash};

use crate::producer::producer;
use crate::stop::Stop;

/// The diff of `old` and `new`, two model-dependency documents as `dependencies` produces them
/// (DEP-4.1). It reads only `model_hash` and the `object` and `definition_hash` of each entry.
///
/// An `object` only in `new` is `added`, only in `old` `removed`, and in both with another
/// `definition_hash` `changed` (DEP-4.2). `changes` is ordered by kind, then by `object`
/// (DEP-4.3): two documents of the same model give `changes: []`.
///
/// # Errors
///
/// A [`CompileErrorDocument`] with an internal error when the build has no valid producer.
pub fn diff(
    old: &ModelDependenciesDocument,
    new: &ModelDependenciesDocument,
) -> Result<ModelDiffDocument, CompileErrorDocument> {
    let producer = producer().map_err(|error| CompileErrorDocument {
        contract: CompileErrorContract::CompileError,
        contract_version: ContractKind::CompileError.latest_version(),
        errors: vec![Stop::from(error).into_diagnostic()],
        warnings: Vec::new(),
    })?;
    let old_hashes = hashes(old);
    let new_hashes = hashes(new);
    let mut changes: Vec<ModelChange> = Vec::new();
    for (object, hash) in &old_hashes {
        match new_hashes.get(object) {
            None => changes.push(change(object, ChangeKind::Removed)),
            Some(new_hash) if new_hash != hash => changes.push(change(object, ChangeKind::Changed)),
            Some(_) => {}
        }
    }
    for object in new_hashes.keys() {
        if !old_hashes.contains_key(object) {
            changes.push(change(object, ChangeKind::Added));
        }
    }
    changes.sort_by(|left, right| compare_objects(&left.object, &right.object));
    Ok(ModelDiffDocument {
        contract: ModelDiffContract::ModelDiff,
        contract_version: ContractKind::ModelDiff.latest_version(),
        producer,
        old_model_hash: old.model_hash.clone(),
        new_model_hash: new.model_hash.clone(),
        changes,
    })
}

fn change(object: &str, kind: ChangeKind) -> ModelChange {
    ModelChange {
        object: object.to_owned(),
        change: kind,
    }
}

/// The `definition_hash` of every `object` of `document`.
fn hashes(document: &ModelDependenciesDocument) -> BTreeMap<&str, &Sha256Hash> {
    let elements = document
        .datasets
        .iter()
        .chain(&document.relationships)
        .chain(&document.fields)
        .chain(&document.policies)
        .map(|entry| (entry.object.as_str(), &entry.definition_hash));
    let metrics = document
        .metrics
        .iter()
        .map(|entry| (entry.object.as_str(), &entry.definition_hash));
    elements.chain(metrics).collect()
}

/// DEP-4.3 order: by kind (`dataset`, `relationship`, `field`, `metric`, `policy`), then by
/// the name parts of `object` in CON-5 order, as DEP-3.2 orders each list.
fn compare_objects(left: &str, right: &str) -> Ordering {
    let key = |object: &str| {
        let (kind, name) = object.split_once(':').unwrap_or((object, ""));
        let rank = match kind {
            "dataset" => 0,
            "relationship" => 1,
            "field" => 2,
            "metric" => 3,
            _ => 4,
        };
        (rank, name.split('.').map(str::to_owned).collect::<Vec<_>>())
    };
    key(left).cmp(&key(right))
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::compare_objects;

    #[test]
    fn dep_4_3_changes_are_ordered_by_kind_then_by_name_parts() {
        assert_eq!(
            compare_objects("field:A.B", "relationship:A"),
            Ordering::Greater
        );
        assert_eq!(compare_objects("metric:A", "metric:A.B"), Ordering::Less);
        assert_eq!(compare_objects("field:A.Z", "field:A_B.A"), Ordering::Less);
    }
}
