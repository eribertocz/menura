//! The model elements a query names, in query order (E2-5.3), which rows 10 to 12c check.

use menura_model::internal_model::NormalizedName;
use menura_representations::bound_query::BoundPath;
use menura_representations::origin::QueryLocation;
use menura_representations::query_tree::{QueryPath, QueryReference, QueryTree};

use crate::authorized_query::ResolvedName;
use crate::elements::{ResolvedElements, reference_of};
use crate::names::{written_expression_reference, written_identifier, written_reference};
use crate::walk::nodes;

/// A model element a query names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NamedTarget {
    /// A field or a metric.
    Member(ResolvedName),
    /// A dataset of a path, or of the SQL form's `FROM` relation.
    Dataset(NormalizedName),
    /// A relationship of a path.
    Relationship(NormalizedName),
}

/// One name of the query, resolved and authorized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedElement {
    /// The element it names.
    pub(crate) target: NamedTarget,
    /// The name as written, for messages.
    pub(crate) written: String,
    /// The element of the query it is written in.
    pub(crate) origin: QueryLocation,
}

/// Every name of `tree` with the element it resolved to in `elements`, in query order: the
/// dataset of the SQL form's `FROM` relation, each column's names in written order, each
/// filter's references in written order, the time grains, the date range, then each path's
/// `from`, `to` and relationships.
pub(crate) fn named_elements(
    tree: &QueryTree,
    elements: &ResolvedElements<'_>,
) -> Vec<NamedElement> {
    let mut named: Vec<NamedElement> = elements
        .relation
        .iter()
        .map(|resolved| NamedElement {
            target: NamedTarget::Dataset(resolved.scope.dataset().clone()),
            written: written_identifier(&resolved.relation.dataset),
            origin: resolved.relation.origin.clone(),
        })
        .collect();
    for column in &elements.columns {
        for read in &column.reads {
            named.push(member(&read.target, read.written.clone(), column.origin));
        }
    }
    for filter in &elements.filters {
        let references = nodes(&filter.filter.expression)
            .into_iter()
            .filter_map(reference_of);
        for (reference, target) in references.zip(&filter.targets) {
            named.push(member(
                target,
                written_expression_reference(reference),
                &filter.filter.origin,
            ));
        }
    }
    for (grain, resolved) in tree.time_grains().iter().zip(&elements.time_grains) {
        named.push(member(
            &resolved.target,
            written_reference(&grain.dimension),
            &grain.origin,
        ));
    }
    if let (Some(range), Some(resolved)) = (tree.date_range(), &elements.date_range) {
        named.push(member(
            &resolved.target,
            written_reference(&range.dimension),
            &range.origin,
        ));
    }
    for (path, bound) in tree.paths().iter().zip(&elements.paths) {
        named.extend(path_elements(path, bound));
    }
    named
}

fn member(target: &ResolvedName, written: String, origin: &QueryLocation) -> NamedElement {
    NamedElement {
        target: NamedTarget::Member(target.clone()),
        written,
        origin: origin.clone(),
    }
}

/// The datasets and relationships of a path entry, in written order.
fn path_elements(path: &QueryPath, bound: &BoundPath) -> Vec<NamedElement> {
    let element = |target: NamedTarget, reference: &QueryReference| NamedElement {
        target,
        written: written_reference(reference),
        origin: path.origin().clone(),
    };
    let mut named = vec![
        element(NamedTarget::Dataset(bound.from().clone()), path.from()),
        element(NamedTarget::Dataset(bound.to().clone()), path.to()),
    ];
    for (reference, relationship) in path.relationships().iter().zip(bound.relationships()) {
        named.push(element(
            NamedTarget::Relationship(relationship.clone()),
            reference,
        ));
    }
    named
}
