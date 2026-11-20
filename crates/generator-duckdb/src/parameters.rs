//! The bound parameters of a statement, numbered in order of first appearance (E6-4.2).

use std::collections::BTreeMap;

use menura_contracts::types::common::{LogicalType, Parameter};
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

/// The parameters of a statement being written.
#[derive(Debug, Default)]
pub(crate) struct Parameters {
    bound: Vec<Parameter>,
    shared: BTreeMap<SharedValue, u64>,
}

impl Parameters {
    /// Binds a new parameter and returns its placeholder, such as `$3`.
    pub(crate) fn bind(&mut self, logical_type: LogicalType, value: Value) -> String {
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
        self.bind(logical_type, value)
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
