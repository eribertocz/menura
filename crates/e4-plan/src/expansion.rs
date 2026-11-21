//! The expansion of a metric into its base components and its expression over them: a
//! derived metric is replaced by the components of its inputs, down to metrics that are not
//! derived, and its formula by the expressions of those inputs (DER-2.1, DER-2.4, DER-4.1).
//!
//! The order of the base components is the order of E3: for a metric that is not derived,
//! its own components in written order (E0G-1.1); for a derived metric, the components of
//! each input in the order its `AGG(…)` terms are written, each input expanded once.

use menura_model::expression::{Expression, Reference};
use menura_model::internal_model::{Component, InternalModel, Metric, NormalizedName};
use menura_representations::bound_query::MetricPath;

use crate::columns::{column_reference, component_column, field_name};
use crate::error::InvariantViolation;
use crate::rewrite::rewrite;

/// A metric as a query or an `AGG(…)` term names it: of the semantic model when `dataset` is
/// `None`, of `dataset` otherwise (DER-5.1.2).
#[derive(Debug, Clone, PartialEq, Eq)]
struct MetricKey {
    dataset: Option<NormalizedName>,
    metric: NormalizedName,
}

impl MetricKey {
    fn of_path(path: &MetricPath) -> Self {
        match path {
            MetricPath::SemanticModel(metric) => Self {
                dataset: None,
                metric: metric.clone(),
            },
            MetricPath::Dataset { dataset, metric } => Self {
                dataset: Some(dataset.clone()),
                metric: metric.clone(),
            },
        }
    }

    fn of_term(reference: &Reference) -> Self {
        Self {
            dataset: reference.qualifier.as_ref().map(field_name),
            metric: field_name(&reference.name),
        }
    }
}

/// One base component of an expanded metric.
pub(crate) struct BaseComponent<'model> {
    /// The component.
    pub(crate) component: &'model Component,
    /// Whether that metric is a dataset metric, whose `source.` references read source
    /// columns (E0D-3).
    pub(crate) in_dataset_metric: bool,
}

/// A metric of the query, expanded.
pub(crate) struct ExpandedMetric<'model> {
    /// Its base components, in expansion order.
    pub(crate) components: Vec<BaseComponent<'model>>,
    /// Its expression over the columns of its components: the column of base component `i`
    /// is [`component_column`] of the metric's position and `i`.
    pub(crate) expression: Expression,
}

/// The metric `path` names, expanded, at `position` among the metrics of the query.
///
/// # Errors
///
/// [`InvariantViolation::UnknownMetric`] for a metric, or an input, the model does not have
/// or did not classify; [`InvariantViolation::DerivedCycle`] for inputs that lead back to a
/// derived metric; [`InvariantViolation::ComponentMismatch`] for a metric without an
/// expression or whose aggregates do not match its components; and the violation `check`
/// returns for a metric it rejects.
pub(crate) fn expand<'model>(
    path: &MetricPath,
    position: usize,
    model: &'model InternalModel,
    check: &impl Fn(&Metric) -> Result<(), InvariantViolation>,
) -> Result<ExpandedMetric<'model>, InvariantViolation> {
    let mut expansion = Expansion {
        model,
        position,
        check,
        chain: Vec::new(),
        expanded: Vec::new(),
        components: Vec::new(),
    };
    let expression = expansion.expand(MetricKey::of_path(path))?;
    Ok(ExpandedMetric {
        components: expansion.components,
        expression,
    })
}

/// The state of one expansion.
struct Expansion<'model, 'check, Check> {
    model: &'model InternalModel,
    position: usize,
    check: &'check Check,
    /// The derived metrics being expanded, outermost first.
    chain: Vec<MetricKey>,
    /// The inputs already expanded, with their expressions.
    expanded: Vec<(MetricKey, Expression)>,
    /// The base components found so far.
    components: Vec<BaseComponent<'model>>,
}

impl<'model, Check> Expansion<'model, '_, Check>
where
    Check: Fn(&Metric) -> Result<(), InvariantViolation>,
{
    /// The expression of the metric `key` over the columns of its base components.
    fn expand(&mut self, key: MetricKey) -> Result<Expression, InvariantViolation> {
        let metric = find(&key, self.model).ok_or(InvariantViolation::UnknownMetric)?;
        let expression = metric
            .expression
            .as_ref()
            .ok_or(InvariantViolation::ComponentMismatch)?;
        if is_derived(expression) {
            self.chain.push(key);
            let formula = rewrite(expression, &mut |node| match node {
                Expression::MetricAggregate(reference) => {
                    self.input(MetricKey::of_term(reference)).map(Some)
                }
                _ => Ok(None),
            })?;
            self.chain.pop();
            return Ok(formula);
        }
        (self.check)(metric)?;
        self.base(metric, key.dataset.is_some(), expression)
    }

    /// The expression of the input `key` of a derived metric, expanded once.
    fn input(&mut self, key: MetricKey) -> Result<Expression, InvariantViolation> {
        if self.chain.contains(&key) {
            return Err(InvariantViolation::DerivedCycle);
        }
        if let Some((_, expression)) = self.expanded.iter().find(|(known, _)| known == &key) {
            return Ok(expression.clone());
        }
        let expression = self.expand(key.clone())?;
        self.expanded.push((key, expression.clone()));
        Ok(expression)
    }

    /// The expression of the metric that is not derived, `metric`, with each aggregate
    /// replaced by the column of its component (E0G-1.1).
    fn base(
        &mut self,
        metric: &'model Metric,
        in_dataset_metric: bool,
        expression: &Expression,
    ) -> Result<Expression, InvariantViolation> {
        let own = &metric
            .classification
            .as_ref()
            .ok_or(InvariantViolation::UnknownMetric)?
            .components;
        let first = self.components.len();
        let mut next = 0;
        let evaluated = rewrite(expression, &mut |node| {
            let Expression::Aggregate { .. } = node else {
                return Ok(None);
            };
            let component = own.get(next).ok_or(InvariantViolation::ComponentMismatch)?;
            self.components.push(BaseComponent {
                component,
                in_dataset_metric,
            });
            let column = component_column(self.position, first + next);
            next += 1;
            column_reference(column.as_str()).map(Some)
        })?;
        if next != own.len() {
            return Err(InvariantViolation::ComponentMismatch);
        }
        Ok(evaluated)
    }
}

/// Whether `expression` reads another metric with `AGG(…)`: the formula of a derived metric.
fn is_derived(expression: &Expression) -> bool {
    let mut found = false;
    let visited: Result<Expression, std::convert::Infallible> = rewrite(expression, &mut |node| {
        found |= matches!(node, Expression::MetricAggregate(_));
        Ok(None)
    });
    drop(visited);
    found
}

/// The metric `key` names in `model`.
fn find<'model>(key: &MetricKey, model: &'model InternalModel) -> Option<&'model Metric> {
    match &key.dataset {
        None => model.metric(&key.metric),
        Some(dataset) => model
            .dataset(dataset)?
            .metrics
            .iter()
            .find(|candidate| candidate.name == key.metric),
    }
}
