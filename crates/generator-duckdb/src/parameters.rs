//! The bound parameters of a statement, numbered in order of first appearance (E6-4.2).

use std::collections::BTreeMap;

use menura_contracts::types::common::{LogicalType, Parameter};
use menura_representations::origin::Origin;
use serde_json::Value;

/// A value bound once and referenced wherever it appears: the query time zone, the reference
/// date and time, or an attribute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SharedValue {
    /// The query `time_zone` (E6-5.2).
    TimeZone,
    /// The reference date, the value of `CURRENT_DATE` (E0F-2.3).
    ReferenceDate,
    /// The reference time as an instant, read by `CURRENT_TIME` and `CURRENT_TIMESTAMP`.
    ReferenceInstant,
    /// `USER_ATTRIBUTE('name')`.
    Attribute(String),
}

/// The query element a node comes from: its JSON Pointer and, for an element of a text such as
/// the SQL of the SQL form, whose elements all share the pointer `/sql`, its line and column in
/// that text.
type Element = (String, Option<(usize, usize)>);

/// One literal occurrence of the query: the query element a node comes from, and the position
/// of the literal among the literals of that node, from 0. Every copy of a query element in the
/// plan, in another branch or node, has the same occurrences (E6-4.2).
type Occurrence = (Element, usize);

/// The parameters of a statement being written.
#[derive(Debug, Default)]
pub(crate) struct Parameters {
    bound: Vec<Parameter>,
    shared: BTreeMap<SharedValue, u64>,
    occurrences: BTreeMap<Occurrence, u64>,
    node_element: Option<Element>,
    node_literals: usize,
}

impl Parameters {
    /// Binds a new parameter and returns its placeholder, such as `$3`.
    fn bind_new(&mut self, logical_type: LogicalType, value: Value) -> String {
        let position = self.next_position();
        self.bound.push(Parameter {
            position,
            logical_type,
            value,
        });
        placeholder(position)
    }

    /// The placeholder of `shared`, bound to `logical_type` and `value` at its first
    /// appearance and reused afterwards.
    pub(crate) fn bind_shared(
        &mut self,
        shared: SharedValue,
        logical_type: LogicalType,
        value: Value,
    ) -> String {
        if let Some(position) = self.shared.get(&shared) {
            return placeholder(*position);
        }
        self.shared.insert(shared, self.next_position());
        self.bind_new(logical_type, value)
    }

    /// Starts the literals of a node that comes from `origin`: the next literal bound with
    /// [`Parameters::bind`] is its first.
    pub(crate) fn start_node(&mut self, origin: &Origin) {
        self.node_element = match origin {
            Origin::Query(location) => Some((
                location.pointer().to_owned(),
                location
                    .position()
                    .map(|position| (position.line(), position.column())),
            )),
            Origin::Model(_) => None,
        };
        self.node_literals = 0;
    }

    /// The placeholder, such as `$3`, of the next query literal of the current node, of
    /// `logical_type` and `value`: the parameter of the same occurrence when an earlier node
    /// bound it to the same value, a new parameter otherwise (E6-4.2).
    pub(crate) fn bind(&mut self, logical_type: LogicalType, value: Value) -> String {
        let ordinal = self.node_literals;
        self.node_literals += 1;
        let Some(element) = self.node_element.clone() else {
            return self.bind_new(logical_type, value);
        };
        let occurrence = (element, ordinal);
        if let Some(position) = self.occurrences.get(&occurrence).copied() {
            let same = self.bound.iter().any(|parameter| {
                parameter.position == position
                    && parameter.logical_type == logical_type
                    && parameter.value == value
            });
            if same {
                return placeholder(position);
            }
        }
        self.occurrences.insert(occurrence, self.next_position());
        self.bind_new(logical_type, value)
    }

    /// The position of `shared`, when the statement binds it.
    pub(crate) fn shared_position(&self, shared: &SharedValue) -> Option<u64> {
        self.shared.get(shared).copied()
    }

    /// The parameters, by position.
    pub(crate) fn into_list(self) -> Vec<Parameter> {
        self.bound
    }

    fn next_position(&self) -> u64 {
        u64::try_from(self.bound.len()).map_or(u64::MAX, |count| count + 1)
    }
}

fn placeholder(position: u64) -> String {
    format!("${position}")
}
