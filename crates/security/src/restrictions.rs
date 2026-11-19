//! The roles every field and metric is restricted to (SCY-3.1, SCY-3.1a, SCY-5.1.8).
//!
//! A restriction protects source columns, not names: the columns a field named in `fields`
//! reads are protected, and every field that reads one of them is restricted to the same roles.
//! Restrictions combine by intersection: a field or metric that several restrictions reach is
//! restricted to the roles every one of them allows. A field or metric that no restriction
//! reaches is not restricted.

use std::collections::{BTreeMap, BTreeSet};

use crate::role::Role;

/// What the restrictions of a model are computed from. `Field`, `Column` and `Metric` identify
/// a field, a source column and a metric; the caller chooses them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestrictionInput<Field, Column, Metric> {
    /// The source columns the expression of each field reads.
    pub field_columns: BTreeMap<Field, BTreeSet<Column>>,
    /// Each field named in an `access.fields` item, with the roles of that item, in the order
    /// written (SCY-3.1).
    pub declared: Vec<(Field, BTreeSet<Role>)>,
    /// The fields the components of each metric read.
    pub metric_fields: BTreeMap<Metric, BTreeSet<Field>>,
    /// The input metrics of each derived or context metric.
    pub metric_inputs: BTreeMap<Metric, BTreeSet<Metric>>,
}

/// The restricted fields and metrics, each with the roles it is restricted to; an element that
/// is absent is not restricted (SCY-5.1.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restrictions<Field, Metric> {
    /// The restricted fields.
    pub fields: BTreeMap<Field, BTreeSet<Role>>,
    /// The restricted metrics.
    pub metrics: BTreeMap<Metric, BTreeSet<Role>>,
}

/// Computes the restriction of every field and metric of `input` (SCY-3.1, SCY-3.1a,
/// SCY-5.1.8).
///
/// - A field is restricted by every declared restriction that names it or that names a field
///   reading a source column it reads, to the intersection of their roles.
/// - A metric is restricted to the intersection of the restrictions of the fields its
///   components read.
/// - A derived or context metric is restricted to the intersection of the restrictions of its
///   inputs. Metric cycles are rejected before restrictions are computed (`E0C-34`); an input
///   met again while its own restriction is being computed adds nothing.
///
/// ```
/// use std::collections::{BTreeMap, BTreeSet};
/// use semantic_compiler_security::{RestrictionInput, Role, restrictions};
///
/// let admin = BTreeSet::from([Role::new("admin")]);
/// let input = RestrictionInput {
///     field_columns: BTreeMap::from([("unit_cost", BTreeSet::from(["cost"])), ("cost_copy", BTreeSet::from(["cost"]))]),
///     declared: vec![("unit_cost", admin.clone())],
///     metric_fields: BTreeMap::from([("total_cost", BTreeSet::from(["cost_copy"]))]),
///     metric_inputs: BTreeMap::<&str, BTreeSet<&str>>::new(),
/// };
/// let found = restrictions(&input);
/// assert_eq!(found.fields.get("cost_copy"), Some(&admin));
/// assert_eq!(found.metrics.get("total_cost"), Some(&admin));
/// ```
#[must_use]
pub fn restrictions<Field, Column, Metric>(
    input: &RestrictionInput<Field, Column, Metric>,
) -> Restrictions<Field, Metric>
where
    Field: Ord + Clone,
    Column: Ord,
    Metric: Ord + Clone,
{
    let fields = field_restrictions(input);
    let mut metrics = BTreeMap::new();
    let mut computer = MetricRestrictions {
        input,
        fields: &fields,
        found: BTreeMap::new(),
        visiting: BTreeSet::new(),
    };
    for metric in input.metric_fields.keys().chain(input.metric_inputs.keys()) {
        if let Some(roles) = computer.restriction(metric) {
            metrics.insert(metric.clone(), roles);
        }
    }
    Restrictions { fields, metrics }
}

/// The restricted fields (SCY-3.1a).
fn field_restrictions<Field, Column, Metric>(
    input: &RestrictionInput<Field, Column, Metric>,
) -> BTreeMap<Field, BTreeSet<Role>>
where
    Field: Ord + Clone,
    Column: Ord,
{
    let mut readers: BTreeMap<&Column, Vec<&Field>> = BTreeMap::new();
    for (field, columns) in &input.field_columns {
        for column in columns {
            readers.entry(column).or_default().push(field);
        }
    }
    let mut restricted: BTreeMap<Field, BTreeSet<Role>> = BTreeMap::new();
    for (declared, roles) in &input.declared {
        let mut reached: BTreeSet<&Field> = BTreeSet::from([declared]);
        for column in input.field_columns.get(declared).into_iter().flatten() {
            reached.extend(readers.get(column).into_iter().flatten().copied());
        }
        for field in reached {
            restrict(&mut restricted, field, roles);
        }
    }
    restricted
}

/// Restricts `element` to `roles` too: the intersection with its earlier restriction.
fn restrict<Element: Ord + Clone>(
    restricted: &mut BTreeMap<Element, BTreeSet<Role>>,
    element: &Element,
    roles: &BTreeSet<Role>,
) {
    match restricted.get_mut(element) {
        Some(current) => current.retain(|role| roles.contains(role)),
        None => {
            restricted.insert(element.clone(), roles.clone());
        }
    }
}

/// The intersection of `restrictions`, or `None` when there is none.
fn intersection<'roles>(
    restrictions: impl IntoIterator<Item = &'roles BTreeSet<Role>>,
) -> Option<BTreeSet<Role>> {
    restrictions.into_iter().fold(None, |found, roles| {
        Some(match found {
            None => roles.clone(),
            Some(current) => current.intersection(roles).cloned().collect(),
        })
    })
}

/// Computes the restrictions of metrics once each, inputs first.
struct MetricRestrictions<'input, Field, Column, Metric> {
    input: &'input RestrictionInput<Field, Column, Metric>,
    fields: &'input BTreeMap<Field, BTreeSet<Role>>,
    found: BTreeMap<&'input Metric, Option<BTreeSet<Role>>>,
    visiting: BTreeSet<&'input Metric>,
}

impl<'input, Field: Ord, Column, Metric: Ord> MetricRestrictions<'input, Field, Column, Metric> {
    /// The restriction of `metric`, or `None` when it is not restricted.
    fn restriction(&mut self, metric: &'input Metric) -> Option<BTreeSet<Role>> {
        if let Some(found) = self.found.get(metric) {
            return found.clone();
        }
        if !self.visiting.insert(metric) {
            return None;
        }
        let mut applicable: Vec<BTreeSet<Role>> = self
            .input
            .metric_fields
            .get(metric)
            .into_iter()
            .flatten()
            .filter_map(|field| self.fields.get(field).cloned())
            .collect();
        for input in self.input.metric_inputs.get(metric).into_iter().flatten() {
            applicable.extend(self.restriction(input));
        }
        let restriction = intersection(&applicable);
        self.visiting.remove(metric);
        self.found.insert(metric, restriction.clone());
        restriction
    }
}

#[cfg(test)]
mod tests;
