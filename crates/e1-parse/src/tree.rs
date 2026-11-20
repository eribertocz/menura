//! The query tree of the wide-table form (E1-5), each element with its origin.

use menura_contracts::types::query::{
    CompilerExtension, DateRange, DimensionItem, MetricItem, OrderItem, PathChoice, TimeGrain,
    WideTableQuery,
};
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;
use menura_representations::query_tree::{
    QueryDateRange, QueryDimension, QueryDimensionKind, QueryFilter, QueryLimit, QueryMetric,
    QueryMetricKind, QueryOrderItem, QueryPath, QueryReference, QueryTree, QueryTreeParts,
    TimeGrainChoice,
};
use menura_representations::time::QueryTime;

use crate::error::{ParseError, ParseErrorCode};
use crate::pointer::{self, DIMENSIONS, METRICS};
use crate::references::split_reference;

/// Builds the tree of a wide-table query whose filters are already parsed. Dimensions come
/// before metrics (E1-5 "Column order").
pub(crate) fn wide_table_tree(
    query: &WideTableQuery,
    time: QueryTime,
    filters: Vec<QueryFilter>,
) -> Result<QueryTree, ParseError> {
    let extension = query.nonportable_extensions.iter().flatten().next();
    let dimensions = query
        .dimensions
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, item)| dimension(index, item))
        .collect::<Result<_, _>>()?;
    let metrics = query
        .metrics
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, item)| metric(index, item))
        .collect::<Result<_, _>>()?;
    let mut parts = match extension {
        Some(extension) => extension_parts(&extension.data)?,
        None => QueryTreeParts::default(),
    };
    parts.filters = filters;
    Ok(QueryTree::wide_table(dimensions, metrics, time, parts))
}

fn dimension(index: usize, item: &DimensionItem) -> Result<QueryDimension, ParseError> {
    let (field, alias) = match item {
        DimensionItem::Reference(field) => (field, None),
        DimensionItem::Aliased(aliased) => (&aliased.field, aliased.alias.as_deref()),
    };
    let origin = pointer::item(DIMENSIONS, index);
    let reference = reference(field, &origin)?;
    Ok(QueryDimension {
        output_name: output_name(alias, &reference),
        kind: QueryDimensionKind::Field { reference },
        origin: at(&origin),
    })
}

fn metric(index: usize, item: &MetricItem) -> Result<QueryMetric, ParseError> {
    let (text, alias) = match item {
        MetricItem::Reference(text) => (text, None),
        MetricItem::Aliased(aliased) => (&aliased.metric, aliased.alias.as_deref()),
    };
    let origin = pointer::item(METRICS, index);
    let reference = reference(text, &origin)?;
    Ok(QueryMetric {
        output_name: output_name(alias, &reference),
        kind: QueryMetricKind::Reference(reference),
        origin: at(&origin),
    })
}

/// The alias as written, or else the last part of the reference (E1-5.1).
fn output_name(alias: Option<&str>, reference: &QueryReference) -> OutputName {
    alias.map_or_else(|| OutputName::from_reference(reference), OutputName::new)
}

/// The extension elements other than filters, as written (QRY-4). E2 matches time grains to
/// dimensions (QRY-4.1).
fn extension_parts(data: &CompilerExtension) -> Result<QueryTreeParts, ParseError> {
    let time_grains = data
        .time_grains
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, entry)| time_grain(index, entry))
        .collect::<Result<_, _>>()?;
    let paths = data
        .paths
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, choice)| path(index, choice))
        .collect::<Result<_, _>>()?;
    let order = data
        .order_by
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, item)| order_item(index, item))
        .collect();
    Ok(QueryTreeParts {
        filters: Vec::new(),
        time_grains,
        date_range: data.date_range.as_ref().map(date_range).transpose()?,
        paths,
        order,
        limit: data.limit.map(|rows| QueryLimit {
            rows,
            origin: at(&pointer::extension_member(0, "limit")),
        }),
    })
}

fn time_grain(index: usize, entry: &TimeGrain) -> Result<TimeGrainChoice, ParseError> {
    let origin = pointer::item(&pointer::extension_member(0, "time_grains"), index);
    Ok(TimeGrainChoice {
        dimension: reference(&entry.dimension, &origin)?,
        grain: entry.grain,
        origin: at(&origin),
    })
}

fn date_range(range: &DateRange) -> Result<QueryDateRange, ParseError> {
    let origin = pointer::extension_member(0, "date_range");
    Ok(QueryDateRange {
        dimension: reference(&range.dimension, &origin)?,
        bounds: range.bounds.clone(),
        origin: at(&origin),
    })
}

fn path(index: usize, choice: &PathChoice) -> Result<QueryPath, ParseError> {
    let origin = pointer::item(&pointer::extension_member(0, "paths"), index);
    let relationships = choice
        .relationships
        .iter()
        .map(|text| reference(text, &origin))
        .collect::<Result<_, _>>()?;
    let from = reference(&choice.from, &origin)?;
    let to = reference(&choice.to, &origin)?;
    QueryPath::new(from, to, relationships, at(&origin))
        .ok_or_else(|| broken_invariant(&origin, "a path has at least one relationship"))
}

fn order_item(index: usize, item: &OrderItem) -> QueryOrderItem {
    QueryOrderItem {
        output: OutputName::new(&item.output),
        direction: item.direction,
        nulls: item.nulls,
        origin: at(&pointer::item(
            &pointer::extension_member(0, "order_by"),
            index,
        )),
    }
}

fn reference(text: &str, origin: &str) -> Result<QueryReference, ParseError> {
    split_reference(text).ok_or_else(|| broken_invariant(origin, "a reference has one part"))
}

fn at(pointer: &str) -> QueryLocation {
    QueryLocation::new(pointer, None)
}

/// A document that passed its JSON Schema but breaks a guarantee of that schema.
fn broken_invariant(pointer: &str, guarantee: &str) -> ParseError {
    ParseError::new(
        ParseErrorCode::InvalidQuery,
        pointer,
        format!("the query does not match its schema: {guarantee}"),
    )
}
