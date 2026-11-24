//! Whether the evaluation of a model or query element needs a value `ClickHouse` cannot
//! express (E6-7.12): a value of such a type anywhere in its expressions, in the fields and
//! source columns they read, and in the metrics they read, or a `date` parameter outside the
//! range of `Date32`.

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::{CatalogSchemaDocument, RelationSchema};
use menura_contracts::types::common::{LogicalType, TimestampPrecision};
use menura_expressions::typing::{Aggregates, Outcome, Stop, Typed, TypingScope, type_expression};
use menura_model::expression::{DateValue, Expression, Identifier, Literal, Reference};
use menura_model::internal_model::{
    Dataset, Field, InternalModel, Metric, NormalizedName, TypedValue, ValueType,
};

use super::walk::{call_operands, subexpressions};
use crate::expression::catalog_column_named;
use crate::types::is_inexpressible_value;

/// How deep metrics may read other metrics before the search stops; E0 rejects cycles, so a
/// real model never reaches it.
const MAXIMUM_DEPTH: usize = 64;

/// The first and last years of `Date32` (E6-7.12).
const DATE_YEARS: (u16, u16) = (1900, 2299);

/// The last day of `DateTime64` with precision `ns` (E6-7.12).
const NANOSECOND_LAST_DAY: (u16, u8, u8) = (2262, 4, 11);

/// What the references of an expression name.
pub(super) enum Scope<'input> {
    /// The columns of the catalog relation of a dataset: a field's expression.
    Catalog(&'input RelationSchema),
    /// The fields of these datasets: a query, metric or row-policy expression.
    Fields(Vec<&'input Dataset>),
}

/// The search, over a model and its catalog schema.
pub(super) struct Needs<'input> {
    /// The model.
    pub(super) model: &'input InternalModel,
    /// Its catalog schema.
    pub(super) catalog: &'input CatalogSchemaDocument,
    /// The reference date of the query, the value of `CURRENT_DATE`.
    pub(super) reference_date: Option<DateValue>,
}

impl<'input> Needs<'input> {
    /// Whether `expression`, read in `scope`, needs such a value. `parameters` says whether its
    /// literals are bound parameters (a query element), whose dates must fit `Date32`.
    pub(super) fn expression(
        &self,
        expression: &Expression,
        scope: &Scope<'input>,
        parameters: bool,
        depth: usize,
    ) -> bool {
        let parts = subexpressions(expression);
        if parts
            .iter()
            .any(|part| self.part(part, scope, parameters, depth))
        {
            return true;
        }
        // A `date` parameter compared with a timestamp of precision `ns` becomes a
        // `DateTime64(9)`, which ends on 2262-04-11.
        parameters
            && parts.iter().any(|part| matches!(part, Expression::Literal(Literal::Date(date)) if after_nanosecond_range(*date)))
            && parts.iter().any(|part| {
                self.type_in(part, scope)
                    .is_some_and(|typed| typed.logical().is_some_and(is_nanosecond_timestamp))
            })
    }

    /// Whether the field `field` of `dataset` needs such a value: its type, or its expression
    /// over the catalog relation of the dataset.
    pub(super) fn field(&self, dataset: &Dataset, field: &Field, depth: usize) -> bool {
        if typed_inexpressible(field.typed.as_ref()) {
            return true;
        }
        let Some(expression) = &field.expression else {
            return false;
        };
        self.relation(dataset).is_some_and(|relation| {
            self.expression(expression, &Scope::Catalog(relation), false, depth)
        })
    }

    /// Whether the metric `metric` needs such a value: its type, its expression, its
    /// components, and the metrics it reads.
    pub(super) fn metric(&self, metric: &Metric, depth: usize) -> bool {
        if depth > MAXIMUM_DEPTH {
            return false;
        }
        if typed_inexpressible(metric.typed.as_ref()) {
            return true;
        }
        let everywhere = Scope::Fields(self.model.parts().datasets.iter().collect());
        let own = metric
            .expression
            .as_ref()
            .is_some_and(|expression| self.expression(expression, &everywhere, false, depth));
        own || self.classified(metric, depth) || self.read_metrics(metric, depth)
    }

    /// Whether a component of `metric` needs such a value.
    fn classified(&self, metric: &Metric, depth: usize) -> bool {
        let Some(classification) = &metric.classification else {
            return false;
        };
        classification.components.iter().any(|component| {
            let scope = Scope::Fields(
                self.model
                    .dataset(&component.home_dataset)
                    .into_iter()
                    .collect(),
            );
            call_operands(&component.call)
                .into_iter()
                .chain(component.filter.as_ref())
                .any(|operand| self.expression(operand, &scope, false, depth))
        })
    }

    /// Whether a metric that `metric` reads, as a derived input or a context base, needs such
    /// a value.
    fn read_metrics(&self, metric: &Metric, depth: usize) -> bool {
        let inputs = metric
            .derived
            .iter()
            .flat_map(|derived| derived.inputs.iter())
            .chain(metric.context.iter().map(|context| &context.base));
        inputs
            .filter_map(|name| self.metric_named(None, name))
            .any(|read| self.metric(read, depth + 1))
    }

    /// The metric `name`, of the dataset `dataset` when given, else of the semantic model or
    /// of any dataset.
    pub(super) fn metric_named(
        &self,
        dataset: Option<&NormalizedName>,
        name: &NormalizedName,
    ) -> Option<&'input Metric> {
        let in_datasets = || {
            self.model
                .parts()
                .datasets
                .iter()
                .filter(|candidate| dataset.is_none_or(|wanted| &candidate.name == wanted))
                .flat_map(|candidate| candidate.metrics.iter())
                .find(|metric| &metric.name == name)
        };
        match dataset {
            Some(_) => in_datasets(),
            None => self.model.metric(name).or_else(in_datasets),
        }
    }

    /// The field `name` of `dataset`.
    pub(super) fn field_named(
        dataset: &'input Dataset,
        name: &NormalizedName,
    ) -> Option<&'input Field> {
        dataset.fields.iter().find(|field| &field.name == name)
    }

    /// Whether one part of an expression needs such a value.
    fn part(
        &self,
        part: &Expression,
        scope: &Scope<'input>,
        parameters: bool,
        depth: usize,
    ) -> bool {
        match part {
            Expression::Literal(Literal::Time(_) | Literal::Timestamp(_))
            | Expression::CurrentTime
            | Expression::CurrentTimestamp => true,
            Expression::Literal(Literal::Date(date)) => parameters && outside_date32(*date),
            Expression::CurrentDate => self.reference_date.is_some_and(outside_date32),
            Expression::MetricAggregate(reference) => {
                let dataset = reference.qualifier.as_ref().map(identifier_name);
                self.metric_named(dataset.as_ref(), &identifier_name(&reference.name))
                    .is_some_and(|metric| self.metric(metric, depth + 1))
            }
            Expression::Reference(reference) => self.reference(reference, scope, depth),
            _ => self.typed(part, scope),
        }
    }

    /// Whether the reference `reference` reads such a value.
    fn reference(&self, reference: &Reference, scope: &Scope<'input>, depth: usize) -> bool {
        match scope {
            Scope::Catalog(relation) => catalog_column_named(relation, reference.name.text())
                .is_some_and(|column| {
                    is_inexpressible_value(&column.logical_type, column.nullable)
                }),
            Scope::Fields(datasets) => self
                .resolve_field(reference, datasets)
                .is_some_and(|(dataset, field)| self.field(dataset, field, depth)),
        }
    }

    /// Whether `part` has a type `ClickHouse` cannot express, as E0.f types it in `scope`.
    fn typed(&self, part: &Expression, scope: &Scope<'input>) -> bool {
        self.type_in(part, scope).is_some_and(|typed| {
            typed
                .logical()
                .is_some_and(|logical_type| is_inexpressible_value(logical_type, typed.nullable))
        })
    }

    /// The type of `part`, as E0.f types it in `scope`.
    fn type_in(&self, part: &Expression, scope: &Scope<'input>) -> Option<Typed> {
        let resolve = |reference: &Reference| -> Outcome<Typed> {
            let found = match scope {
                Scope::Catalog(relation) => catalog_column_named(relation, reference.name.text())
                    .map(|column| Typed::value(column.logical_type.clone(), column.nullable)),
                Scope::Fields(datasets) => self
                    .resolve_field(reference, datasets)
                    .and_then(|(_, field)| field.typed.as_ref())
                    .and_then(|typed| match &typed.value_type {
                        ValueType::Logical(logical_type) => {
                            Some(Typed::value(logical_type.clone(), typed.nullable))
                        }
                        ValueType::Null => None,
                    }),
            };
            found.ok_or(Stop::ReportedElsewhere)
        };
        let typer = TypingScope::new(&resolve, Aggregates::Allowed);
        type_expression(part, &typer).ok()
    }

    /// The dataset and field a reference names among `datasets`: the dataset of its
    /// qualifier, or the first that has the field.
    fn resolve_field(
        &self,
        reference: &Reference,
        datasets: &[&'input Dataset],
    ) -> Option<(&'input Dataset, &'input Field)> {
        let name = identifier_name(&reference.name);
        let qualifier = reference.qualifier.as_ref().map(identifier_name);
        let candidates: Vec<&'input Dataset> = match &qualifier {
            Some(dataset) => self.model.dataset(dataset).into_iter().collect(),
            None => datasets.to_vec(),
        };
        candidates
            .into_iter()
            .find_map(|dataset| Self::field_named(dataset, &name).map(|field| (dataset, field)))
    }

    /// The catalog relation of `dataset`.
    fn relation(&self, dataset: &Dataset) -> Option<&'input RelationSchema> {
        self.catalog
            .relations
            .iter()
            .find(|relation| relation.source == dataset.source)
    }
}

/// Whether a declared type has no `ClickHouse` type.
fn typed_inexpressible(typed: Option<&TypedValue>) -> bool {
    typed.is_some_and(|typed| match &typed.value_type {
        ValueType::Logical(logical_type) => is_inexpressible_value(logical_type, typed.nullable),
        ValueType::Null => false,
    })
}

/// Whether `date` lies outside the range of `Date32`, 1900-01-01 to 2299-12-31 (E6-7.12).
pub(super) fn outside_date32(date: DateValue) -> bool {
    date.year() < DATE_YEARS.0 || date.year() > DATE_YEARS.1
}

/// Whether `date` lies after 2262-04-11, the last day of `DateTime64` with precision `ns`.
pub(super) fn after_nanosecond_range(date: DateValue) -> bool {
    (date.year(), date.month(), date.day()) > NANOSECOND_LAST_DAY
}

/// Whether `logical_type` is a timestamp of precision `ns`.
pub(super) fn is_nanosecond_timestamp(logical_type: &LogicalType) -> bool {
    matches!(
        logical_type,
        LogicalType::Timestamp {
            precision: TimestampPrecision::Ns,
            ..
        }
    )
}

/// The normalized name of an identifier (E0D-1.1): a regular identifier in upper case, a
/// delimited one exactly.
pub(super) fn identifier_name(identifier: &Identifier) -> NormalizedName {
    if identifier.is_delimited() {
        NormalizedName::new(identifier.text())
    } else {
        NormalizedName::new(&normalize_unquoted_identifier(identifier.text()))
    }
}

#[cfg(test)]
mod tests {
    use menura_model::expression::DateValue;

    use super::{after_nanosecond_range, outside_date32};

    fn date(year: u16, month: u8, day: u8) -> DateValue {
        DateValue::new(year, month, day).expect("a date")
    }

    #[test]
    fn e6_7_12_date_parameters_must_fit_date32_and_nanosecond_timestamps() {
        assert!(outside_date32(date(1899, 12, 31)));
        assert!(!outside_date32(date(1900, 1, 1)));
        assert!(!outside_date32(date(2299, 12, 31)));
        assert!(outside_date32(date(2300, 1, 1)));
        assert!(!after_nanosecond_range(date(2262, 4, 11)));
        assert!(after_nanosecond_range(date(2262, 4, 12)));
    }
}
