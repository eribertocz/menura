//! Context queries (E4-8.2): the bound query of each context query, found in the order of
//! E4-8.8 and shared by the context metrics with the same base and normalized modifier.
//!
//! A context query is planned by E3, E4 and E5 like a query (E4-8.2). E4 builds its bound
//! query here, the caller resolves its branches with E3, and [`crate::build_plan_with_contexts`]
//! plans it and attaches it to the result rows with a `Lookup` (E4-8.7).

use std::collections::BTreeSet;

use menura_contracts::types::common::Grain;
use menura_model::internal_model::{
    ContextModifier, FieldPath, InternalModel, Metric, NormalizedName, RemovedDimensions,
    SourceLocation,
};
use menura_representations::bound_query::{
    BoundDimension, BoundDimensionKind, BoundMetric, BoundMetricKind, BoundQuery, BoundQueryParts,
    MetricPath,
};
use menura_representations::columns::Columns;
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;
use menura_representations::paths::Branches;

use crate::error::InvariantViolation;
use crate::to_date::{ToDatePlan, to_date_plan};

/// One context query of a query (E4-8.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextQuery {
    /// The bound query Q<sub>C</sub>: the query with the base as its only metric, without
    /// `order_by` or `limit`, changed for the modifier (E4-8.3).
    query: BoundQuery,
    /// What makes two context metrics share it (E4-8.2).
    key: ContextKey,
    /// The first context metric that needs it (E4-8.8).
    metric: NormalizedName,
    /// Where that metric is written: the origin of its `Lookup` (E6-4.12).
    location: SourceLocation,
    /// The positions, among the query's dimensions, of the dimensions it keeps, in order:
    /// its dimension `i` is the query dimension `kept[i]` (E4-8.3).
    kept: Vec<usize>,
    /// For `period_to_date`, the time key of its branches (E4-8.5, E4-8.6).
    to_date: Option<TimeKey>,
    /// Where the query metric that reads the first context metric is written.
    reader: QueryLocation,
}

impl ContextQuery {
    /// The bound query of the context query, which E3 resolves like a query (E4-8.2).
    #[must_use]
    pub fn query(&self) -> &BoundQuery {
        &self.query
    }

    /// Takes the `paths` entry written at `origin` out of the context query, so that it keeps
    /// only the query's entries its own branches use (E4-8.2). Returns whether it had one.
    pub fn without_path(&mut self, origin: &QueryLocation) -> bool {
        let paths = self.query.paths();
        let Some(position) = paths.iter().position(|path| path.origin() == origin) else {
            return false;
        };
        let mut kept = paths.to_vec();
        kept.remove(position);
        let parts = BoundQueryParts {
            filters: self.query.filters().to_vec(),
            date_range: self.query.date_range().cloned(),
            paths: kept,
            order: Vec::new(),
            limit: None,
            authorization: self.query.authorization().to_vec(),
            subgraph: self.query.subgraph().clone(),
        };
        self.query = BoundQuery::new(
            self.query.columns().clone(),
            self.query.time().clone(),
            parts,
        );
        true
    }

    /// Whether `other` is this context query, its `paths` aside: the same base, normalized
    /// modifier and first context metric.
    pub(crate) fn is_same_context(&self, other: &ContextQuery) -> bool {
        self.key == other.key && self.metric == other.metric && self.kept == other.kept
    }

    /// The first context metric that needs it, in the order of E4-8.8.
    pub(crate) fn metric(&self) -> &NormalizedName {
        &self.metric
    }

    /// Where the first context metric that needs it is written.
    pub(crate) fn location(&self) -> &SourceLocation {
        &self.location
    }

    /// The positions of the query dimensions it keeps, in its dimension order.
    pub(crate) fn kept(&self) -> &[usize] {
        &self.kept
    }

    /// The modifier type, as `reason` names it (PLN-7.3).
    pub(crate) fn modifier_type(&self) -> &'static str {
        match self.key.modifier {
            NormalizedModifier::ShareOfTotal(_) => "share_of_total",
            NormalizedModifier::PeriodToDate { .. } => "period_to_date",
        }
    }

    /// For `period_to_date`, the `ToDate` of its branches, or `Err` with the location of
    /// `E4-02` (E4-8.6); `Ok(None)` for another modifier.
    pub(crate) fn to_date(&self) -> Result<Option<&ToDatePlan>, &QueryLocation> {
        match &self.to_date {
            None => Ok(None),
            Some(TimeKey::Found(plan)) => Ok(Some(plan)),
            Some(TimeKey::Missing) => Err(&self.reader),
        }
    }

    /// The name of the base metric.
    pub(crate) fn base(&self) -> &NormalizedName {
        match &self.key.base {
            MetricPath::SemanticModel(name) | MetricPath::Dataset { metric: name, .. } => name,
        }
    }
}

/// The time key of a `period_to_date` context query (E4-8.6).
#[derive(Debug, Clone, PartialEq, Eq)]
enum TimeKey {
    /// The query has it: the branches repeat their rows by this `ToDate`.
    Found(ToDatePlan),
    /// The query lacks it: `E4-02` at the query metric that reads the context metric.
    Missing,
}

/// A context query with the branches E3 resolved for it.
#[derive(Debug, Clone, Copy)]
pub struct ContextBranches<'input> {
    /// The context query.
    pub context: &'input ContextQuery,
    /// Its branches (E3-4).
    pub branches: &'input Branches,
}

/// What two context metrics share a context query by: the same base and the same normalized
/// modifier (E4-8.2).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ContextKey {
    base: MetricPath,
    modifier: NormalizedModifier,
}

/// A modifier, normalized as E4-8.2 compares it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NormalizedModifier {
    /// `share_of_total`: the positions of the query dimensions it keeps, so that two `remove`
    /// values that take out the same query dimensions, `all` and a list of every query
    /// dimension included, compare equal (E4-8.2, CTX-3a.5).
    ShareOfTotal(Vec<usize>),
    /// `period_to_date`: its time dimension and grain.
    PeriodToDate {
        /// The time dimension.
        time_dimension: FieldPath,
        /// The grain of the period.
        grain: Grain,
    },
}

/// The context queries of one query, in the order of E4-8.8, as the expansion of its metrics
/// finds them.
pub(crate) struct ContextRegistry<'query> {
    query: &'query BoundQuery,
    model: &'query InternalModel,
    found: Vec<ContextQuery>,
}

impl<'query> ContextRegistry<'query> {
    /// A registry with no context query yet, for the metrics of `query` over `model`.
    pub(crate) fn new(query: &'query BoundQuery, model: &'query InternalModel) -> Self {
        Self {
            query,
            model,
            found: Vec::new(),
        }
    }

    /// The context queries found, in the order of E4-8.8.
    pub(crate) fn into_queries(self) -> Vec<ContextQuery> {
        self.found
    }

    /// The position of the context query of `metric`, a context metric over `base` read by the
    /// query metric written at `origin`: the one an earlier context metric with the same base
    /// and normalized modifier needs, or a new one after the others (E4-8.2, E4-8.8).
    ///
    /// # Errors
    ///
    /// [`InvariantViolation::ContextMetric`] for a metric without the context extension, or
    /// with a modifier E4 does not plan yet.
    pub(crate) fn register(
        &mut self,
        metric: &Metric,
        base: MetricPath,
        origin: &QueryLocation,
    ) -> Result<usize, InvariantViolation> {
        let context = metric
            .context
            .as_ref()
            .ok_or(InvariantViolation::ContextMetric)?;
        let modifier = match &context.modifier {
            ContextModifier::ShareOfTotal(removed) => {
                NormalizedModifier::ShareOfTotal(self.kept(removed))
            }
            ContextModifier::PeriodToDate {
                time_dimension,
                grain,
            } => NormalizedModifier::PeriodToDate {
                time_dimension: time_dimension.clone(),
                grain: *grain,
            },
            ContextModifier::PriorPeriod { .. } => {
                return Err(InvariantViolation::ContextMetric);
            }
        };
        let key = ContextKey { base, modifier };
        if let Some(position) = self.found.iter().position(|found| found.key == key) {
            return Ok(position);
        }
        let all: Vec<usize> = (0..self.query.columns().dimensions().count()).collect();
        let (kept, to_date, without_range) = match &key.modifier {
            NormalizedModifier::ShareOfTotal(kept) => (kept.clone(), None, None),
            NormalizedModifier::PeriodToDate {
                time_dimension,
                grain,
            } => (
                all,
                Some(
                    to_date_plan(self.query, self.model, time_dimension, *grain)
                        .map_or(TimeKey::Missing, TimeKey::Found),
                ),
                Some(time_dimension),
            ),
        };
        let query = self.context_query(&key.base, &kept, without_range, origin);
        self.found.push(ContextQuery {
            query,
            metric: metric.name.clone(),
            location: metric.location.clone(),
            kept,
            to_date,
            reader: origin.clone(),
            key,
        });
        Ok(self.found.len() - 1)
    }

    /// The positions of the query dimensions that a `share_of_total` with the `remove` value
    /// `removed` keeps: none for `all`, otherwise every dimension that is not a field it
    /// names; a `remove` field that is not a query dimension has no effect (E4-8.3, CTX-3a.5).
    fn kept(&self, removed: &RemovedDimensions) -> Vec<usize> {
        let RemovedDimensions::Fields(fields) = removed else {
            return Vec::new();
        };
        let fields: BTreeSet<&FieldPath> = fields.iter().collect();
        self.query
            .columns()
            .dimensions()
            .enumerate()
            .filter(|(_, dimension)| {
                dimension_field(dimension).is_none_or(|field| !fields.contains(field))
            })
            .map(|(position, _)| position)
            .collect()
    }

    /// The bound query of a context query over `base` that keeps the query dimensions at
    /// `kept`: the query's filters, `paths` and authorization unchanged, no `order_by` and no
    /// `limit`, and its `date_range` unless it is on `without_range` (E4-8.2, E4-8.3,
    /// E4-8.5). Its metric is written where the query metric that reads the context metric
    /// is, at `origin`.
    fn context_query(
        &self,
        base: &MetricPath,
        kept: &[usize],
        without_range: Option<&FieldPath>,
        origin: &QueryLocation,
    ) -> BoundQuery {
        let dimensions: Vec<BoundDimension> = self
            .query
            .columns()
            .dimensions()
            .enumerate()
            .filter(|(position, _)| kept.contains(position))
            .map(|(_, dimension)| dimension.clone())
            .collect();
        let metric = BoundMetric {
            kind: BoundMetricKind::Reference(base.clone()),
            output_name: OutputName::new(metric_name(base).as_str()),
            origin: origin.clone(),
        };
        let parts = BoundQueryParts {
            filters: self.query.filters().to_vec(),
            date_range: self
                .query
                .date_range()
                .filter(|range| Some(&range.field) != without_range)
                .cloned(),
            paths: self.query.paths().to_vec(),
            order: Vec::new(),
            limit: None,
            authorization: self.query.authorization().to_vec(),
            subgraph: self.query.subgraph().clone(),
        };
        BoundQuery::new(
            Columns::dimensions_then_metrics(dimensions, vec![metric]),
            self.query.time().clone(),
            parts,
        )
    }
}

/// The field a dimension reads, when it is a field dimension.
fn dimension_field(dimension: &BoundDimension) -> Option<&FieldPath> {
    match &dimension.kind {
        BoundDimensionKind::Field { field, .. } => Some(field),
        BoundDimensionKind::Expression { .. } => None,
    }
}

/// The name of the metric at `path`.
fn metric_name(path: &MetricPath) -> &NormalizedName {
    match path {
        MetricPath::SemanticModel(name) | MetricPath::Dataset { metric: name, .. } => name,
    }
}
