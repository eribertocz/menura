//! The elements of the selected semantic model, found by name.

use std::collections::HashMap;

use semantic_compiler_model::json::JsonForm;

use crate::names::Name;
use crate::outline::{DatasetOutline, ElementOutline, ModelOutline};
use crate::pointer::JsonPointer;

/// The datasets, fields and metrics of a model outline, by normalized name. When two elements
/// share a name the first one is kept; E0.d rejects such a model anyway (`E0D-01`).
pub(super) struct ModelIndex<'model, 'document> {
    datasets: HashMap<&'model Name, &'model DatasetOutline<'document>>,
    metrics: HashMap<&'model Name, &'model ElementOutline<'document>>,
    names: HashMap<&'model JsonPointer, &'model Name>,
}

impl<'model, 'document> ModelIndex<'model, 'document> {
    /// The index of `model`.
    pub(super) fn new(model: &'model ModelOutline<'document>) -> Self {
        let mut index = Self {
            datasets: HashMap::new(),
            metrics: HashMap::new(),
            names: HashMap::new(),
        };
        for dataset in &model.datasets {
            if let Some(name) = &dataset.name {
                index.datasets.entry(&name.name).or_insert(dataset);
                index.names.insert(&dataset.pointer, &name.name);
            }
        }
        for metric in &model.metrics {
            if let Some(name) = &metric.name {
                index.metrics.entry(&name.name).or_insert(metric);
                index.names.insert(&metric.pointer, &name.name);
            }
        }
        index
    }

    /// The dataset named `name`.
    pub(super) fn dataset(&self, name: &Name) -> Option<&'model DatasetOutline<'document>> {
        self.datasets.get(name).copied()
    }

    /// The JSON Pointer of the field `field` of the dataset `dataset`.
    pub(super) fn field(&self, dataset: &Name, field: &Name) -> Option<&'model JsonPointer> {
        self.dataset(dataset)?
            .field(field)
            .map(|element| &element.pointer)
    }

    /// The JSON Pointer of the metric of the semantic model named `name`.
    pub(super) fn metric(&self, name: &Name) -> Option<&'model JsonPointer> {
        self.metrics.get(name).map(|metric| &metric.pointer)
    }

    /// The name of the dataset or metric of the semantic model at `pointer`.
    pub(super) fn name_at(&self, pointer: &JsonPointer) -> Option<&'model Name> {
        self.names.get(pointer).copied()
    }
}

/// The names a tree reads, from its JSON form: the `(qualifier, name)` of every column or field
/// reference and the metric of every `AGG(…)`, normalized, in the order written.
#[derive(Debug, Default)]
pub(super) struct ReadNames {
    /// The column and field references.
    pub(super) references: Vec<(Option<Name>, Name)>,
    /// The metrics read with `AGG(…)`.
    pub(super) metrics: Vec<Name>,
}

impl ReadNames {
    /// The names `tree`, the JSON form of a normalized tree, reads.
    pub(super) fn of(tree: &JsonForm) -> Self {
        let mut names = Self::default();
        names.collect(tree);
        names
    }

    fn collect(&mut self, form: &JsonForm) {
        match form {
            JsonForm::Object(members) => {
                let node = members.get("node");
                if node == Some(&JsonForm::text("reference")) {
                    if let Some(name) = members.get("name").and_then(reference) {
                        self.references.push(name);
                    }
                    return;
                }
                if node == Some(&JsonForm::text("metric_aggregate")) {
                    if let Some((_, name)) = members.get("metric").and_then(reference) {
                        self.metrics.push(name);
                    }
                    return;
                }
                for member in members.values() {
                    self.collect(member);
                }
            }
            JsonForm::List(items) => {
                for item in items {
                    self.collect(item);
                }
            }
            _ => {}
        }
    }
}

/// The normalized `(qualifier, name)` of the JSON form of a reference.
fn reference(form: &JsonForm) -> Option<(Option<Name>, Name)> {
    let JsonForm::Object(members) = form else {
        return None;
    };
    let name = match members.get("name") {
        Some(JsonForm::Text(text)) => normalized_name(text),
        _ => return None,
    };
    let qualifier = match members.get("qualifier") {
        Some(JsonForm::Text(text)) => Some(normalized_name(text)),
        _ => None,
    };
    Some((qualifier, name))
}

/// The name whose normalized text is `text`: a delimited identifier keeps its text exactly.
fn normalized_name(text: &str) -> Name {
    Name::declared(&format!("\"{}\"", text.replace('"', "\"\"")))
}
