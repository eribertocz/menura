//! `E0C-67` to `E0C-69`: the names a temporal relationship declares (VAL-2.2 to VAL-2.4).
//!
//! `from_time` names a field of the relationship's `from` dataset, checked only when `from`
//! resolves; `population` names a `validity` population of its `to` dataset, checked only when
//! `to` resolves; and that population's `entity` holds the same set of columns as
//! `to_columns`, compared as E0E-3.8 compares a column list with a key, checked only when every
//! name of both resolves. Names are declared names compared under E0D-1.3.

use std::collections::BTreeSet;

use menura_contracts::types::catalog::CatalogSchemaDocument;

use super::scope::Scope;
use crate::extensions::populations::PopulationMode;
use crate::extensions::temporal::TemporalDeclaration;
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Population};
use crate::names::Name;
use crate::outline::{DatasetOutline, RelationshipOutline, outline};
use crate::structure::Structure;

/// Resolves the names of every temporal declaration of `extensions`, in the semantic model
/// that `structure` selected, against `catalog`, the admitted catalog schema.
///
/// # Errors
///
/// Returns the `E0C-67`, `E0C-68` and `E0C-69` diagnostics, at most one of each per
/// declaration, at the `data` member of the relationship's entry (E0C-2.6), in document order
/// then by code. Each rejects the model (E0-4).
pub fn resolve_temporal_references(
    structure: &Structure<'_>,
    catalog: &CatalogSchemaDocument,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let scope = Scope::new(&model, catalog, &structure.expressions);
    let errors: Vec<ExtensionDiagnostic> = extensions
        .temporal
        .iter()
        .flat_map(|declaration| declaration_problems(&scope, extensions, declaration))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The diagnostics of `declaration`, in the order of their codes.
fn declaration_problems(
    scope: &Scope<'_, '_>,
    extensions: &Extensions,
    declaration: &TemporalDeclaration,
) -> Vec<ExtensionDiagnostic> {
    let Some(relationship) = scope
        .model
        .relationships
        .iter()
        .find(|relationship| relationship.pointer == declaration.relationship)
    else {
        return Vec::new();
    };
    let end = |end: &Option<crate::outline::DeclaredName<'_>>| {
        end.as_ref().and_then(|name| scope.dataset(&name.name))
    };
    let mut problems = Vec::new();
    if let Some(from) = end(&relationship.from) {
        problems.extend(from_time_problem(from, declaration));
    }
    if let Some(to) = end(&relationship.to) {
        match validity_population(extensions, to, declaration) {
            Some(population) => {
                problems.extend(entity_problem(scope, to, relationship, population));
            }
            None => problems.push((
                ExtensionCode::TemporalPopulation,
                format!(
                    "`{}` is not a validity population of `{}`",
                    declaration.population,
                    declared_text(to)
                ),
            )),
        }
    }
    problems
        .into_iter()
        .map(|(code, message)| ExtensionDiagnostic {
            code,
            pointer: declaration.data.clone(),
            location: declaration.location,
            message,
        })
        .collect()
}

/// `E0C-67` when `from_time` names no field of `from` (VAL-2.2).
fn from_time_problem(
    from: &DatasetOutline<'_>,
    declaration: &TemporalDeclaration,
) -> Option<(ExtensionCode, String)> {
    from.field(&Name::declared(&declaration.from_time))
        .is_none()
        .then(|| {
            (
                ExtensionCode::MissingFromTime,
                format!(
                    "the temporal from_time `{}` is not a field of `{}`",
                    declaration.from_time,
                    declared_text(from)
                ),
            )
        })
}

/// The population of `to` that `declaration` names, when its mode is `validity` (VAL-2.3).
fn validity_population<'extensions>(
    extensions: &'extensions Extensions,
    to: &DatasetOutline<'_>,
    declaration: &TemporalDeclaration,
) -> Option<&'extensions Population> {
    let wanted = Name::declared(&declaration.population);
    extensions.populations.iter().find(|population| {
        population.dataset == to.pointer
            && Name::declared(&population.name) == wanted
            && matches!(population.mode, PopulationMode::Validity { .. })
    })
}

/// `E0C-69` when the `entity` of `population` and the `to_columns` of `relationship` are not
/// the same set of names (VAL-2.4, E0E-3.8); nothing when a name of either does not resolve.
fn entity_problem(
    scope: &Scope<'_, '_>,
    to: &DatasetOutline<'_>,
    relationship: &RelationshipOutline<'_>,
    population: &Population,
) -> Option<(ExtensionCode, String)> {
    let entity: BTreeSet<Name> = population
        .entity
        .iter()
        .map(|field| Name::declared(field))
        .collect();
    let to_columns: BTreeSet<Name> = relationship
        .to_columns
        .iter()
        .map(|column| column.name.clone())
        .collect();
    let entity_resolves = entity.iter().all(|name| to.field(name).is_some());
    let columns_resolve = relationship.to_columns.iter().all(|column| {
        to.field(&column.name).is_some()
            || scope
                .columns(to)
                .and_then(|columns| columns.find(&column.name, column.delimited))
                .is_some()
    });
    (entity_resolves && columns_resolve && entity != to_columns).then(|| {
        (
            ExtensionCode::TemporalEntityKeys,
            format!(
                "the entity of the population `{}` is not the set of the to_columns of the relationship",
                population.name
            ),
        )
    })
}

/// The name of `dataset` as written, for messages.
fn declared_text<'document>(dataset: &DatasetOutline<'document>) -> &'document str {
    dataset.name.as_ref().map_or("", |name| name.written.text)
}
