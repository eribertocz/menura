//! The expansion of a metric into its base components and its expression over them: a
//! derived metric is replaced by the components of its inputs, down to metrics that are not
//! derived, and its formula by the expressions of those inputs (DER-2.1, DER-2.4, DER-4.1).
//!
//! The order of the base components is the order of E3: for a metric that is not derived,
//! its own components in written order (E0G-1.1); for a derived metric, the components of
//! each input in the order its `AGG(…)` terms are written, each input expanded once.
//!
//! Each division follows the rule of the derived metric whose formula writes it (DER-3.1), so
//! a formula never inlines another formula: an input whose value is more than one component
//! column is computed as an input column of its own, one level of derivation per `Project`
//! below the final one (E6-4.6a).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Reference};
use menura_model::internal_model::{Component, InternalModel, Metric, NormalizedName, ValueType};
use menura_representations::bound_query::MetricPath;
use menura_representations::logical_plan::DerivedFormula;
use menura_representations::names::ColumnName;

use crate::columns::{column_reference, component_column, field_name, input_column};
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

/// The value of an input of a derived metric, computed in a `Project` below the final one.
pub(crate) struct InputColumn {
    /// The `Project` that computes it: 0 for the first above the components, and one more
    /// than the highest level of the input columns its expression reads.
    pub(crate) level: usize,
    /// Its column.
    pub(crate) name: ColumnName,
    /// Its expression over the component columns and the input columns of lower levels.
    pub(crate) expression: Expression,
    /// The derived metric whose formula it is, when the input is a derived metric with
    /// `on_division_by_zero` (DER-3.1).
    pub(crate) derived: Option<DerivedFormula>,
    /// Its result type, as E0.f typed the input metric.
    pub(crate) logical_type: LogicalType,
    /// Whether it can be NULL.
    pub(crate) nullable: bool,
}

/// A metric of the query, expanded.
pub(crate) struct ExpandedMetric<'model> {
    /// Its base components, in expansion order.
    pub(crate) components: Vec<BaseComponent<'model>>,
    /// The input columns it reads, in the order they are found.
    pub(crate) inputs: Vec<InputColumn>,
    /// Its expression over the columns of its components and inputs: the column of base
    /// component `i` is [`component_column`] of the metric's position and `i`.
    pub(crate) expression: Expression,
    /// The derived metric whose formula it is, when it is a derived metric with
    /// `on_division_by_zero` (DER-3.1).
    pub(crate) derived: Option<DerivedFormula>,
}

/// The metric `path` names, expanded, at `position` among the metrics of the query.
///
/// # Errors
///
/// [`InvariantViolation::UnknownMetric`] for a metric, or an input, the model does not have
/// or did not classify; [`InvariantViolation::DerivedCycle`] for inputs that lead back to a
/// derived metric; [`InvariantViolation::ComponentMismatch`] for a metric without an
/// expression or whose aggregates do not match its components;
/// [`InvariantViolation::UntypedExpression`] for an input without a type; and the violation
/// `check` returns for a metric it rejects.
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
        inputs: Vec::new(),
    };
    let key = MetricKey::of_path(path);
    let metric = find(&key, model).ok_or(InvariantViolation::UnknownMetric)?;
    let value = expansion.expand(key)?;
    Ok(ExpandedMetric {
        components: expansion.components,
        inputs: expansion.inputs,
        expression: value.expression,
        derived: derived_formula(metric),
    })
}

/// The value of a metric: its expression, and the highest level of the input columns it
/// reads, if any.
#[derive(Clone)]
struct Value {
    expression: Expression,
    reads: Option<usize>,
}

/// The state of one expansion.
struct Expansion<'model, 'check, Check> {
    model: &'model InternalModel,
    position: usize,
    check: &'check Check,
    /// The derived metrics being expanded, outermost first.
    chain: Vec<MetricKey>,
    /// The inputs already expanded, with their values.
    expanded: Vec<(MetricKey, Value)>,
    /// The base components found so far.
    components: Vec<BaseComponent<'model>>,
    /// The input columns found so far.
    inputs: Vec<InputColumn>,
}

impl<'model, Check> Expansion<'model, '_, Check>
where
    Check: Fn(&Metric) -> Result<(), InvariantViolation>,
{
    /// The value of the metric `key` over the columns of its components and inputs.
    fn expand(&mut self, key: MetricKey) -> Result<Value, InvariantViolation> {
        let metric = find(&key, self.model).ok_or(InvariantViolation::UnknownMetric)?;
        let expression = metric
            .expression
            .as_ref()
            .ok_or(InvariantViolation::ComponentMismatch)?;
        (self.check)(metric)?;
        if !is_derived(expression) {
            let expression = self.base(metric, key.dataset.is_some(), expression)?;
            return Ok(Value {
                expression,
                reads: None,
            });
        }
        self.chain.push(key);
        let mut reads = None;
        let formula = rewrite(expression, &mut |node| match node {
            Expression::MetricAggregate(reference) => {
                let input = self.input(MetricKey::of_term(reference))?;
                reads = reads.max(input.reads);
                Ok(Some(input.expression))
            }
            _ => Ok(None),
        })?;
        self.chain.pop();
        Ok(Value {
            expression: formula,
            reads,
        })
    }

    /// The value of the input `key` of a derived metric, expanded once: one component column
    /// is read directly, and any other value is computed as an input column of its own.
    fn input(&mut self, key: MetricKey) -> Result<Value, InvariantViolation> {
        if self.chain.contains(&key) {
            return Err(InvariantViolation::DerivedCycle);
        }
        if let Some((_, value)) = self.expanded.iter().find(|(known, _)| known == &key) {
            return Ok(value.clone());
        }
        let metric = find(&key, self.model).ok_or(InvariantViolation::UnknownMetric)?;
        let value = self.expand(key.clone())?;
        let value = if matches!(value.expression, Expression::Reference(_)) {
            value
        } else {
            self.input_column(metric, value)?
        };
        self.expanded.push((key, value.clone()));
        Ok(value)
    }

    /// The input column that computes `value`, the value of the input `metric`, and the
    /// reference to it.
    fn input_column(&mut self, metric: &Metric, value: Value) -> Result<Value, InvariantViolation> {
        let typed = metric
            .typed
            .as_ref()
            .ok_or(InvariantViolation::UntypedExpression)?;
        let ValueType::Logical(logical_type) = &typed.value_type else {
            return Err(InvariantViolation::UntypedExpression);
        };
        let level = value.reads.map_or(0, |read| read + 1);
        let name = input_column(self.position, self.inputs.len());
        let reference = column_reference(name.as_str())?;
        self.inputs.push(InputColumn {
            level,
            name,
            expression: value.expression,
            derived: derived_formula(metric),
            logical_type: logical_type.clone(),
            nullable: typed.nullable,
        });
        Ok(Value {
            expression: reference,
            reads: Some(level),
        })
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

/// The derived formula of `metric`: its name and `on_division_by_zero`, when it is a derived
/// metric that declares one (DER-3.1). A development-format metric that reads dataset metrics
/// declares none, and its divisions follow E0F-6.1.
fn derived_formula(metric: &Metric) -> Option<DerivedFormula> {
    let rule = metric.derived.as_ref()?.on_division_by_zero?;
    Some(DerivedFormula {
        metric: metric.name.clone(),
        on_division_by_zero: rule,
    })
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
