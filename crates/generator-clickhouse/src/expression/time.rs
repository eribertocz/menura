//! Time grains (E6-7.6) and the types the generator needs to write them.

use menura_contracts::types::common::LogicalType;
use menura_contracts::types::descriptions::Binding;
use menura_expressions::typing::{Aggregates, Outcome, Stop, Typed, TypingScope, type_expression};
use menura_model::expression::{DateUnit, Expression, Literal, Reference};

use super::{Columns, ExpressionWriter, catalog_column};
use crate::error::{GenerateError, InvariantViolation};
use crate::parameters::SharedValue;
use crate::types::{attribute_logical_type, digits};

impl ExpressionWriter<'_, '_> {
    /// `x` truncated to `unit` in the query time zone (E6-7.6, E6-4.10). A date is truncated
    /// with the `ClickHouse` truncation function of its grain, ISO weeks starting on Monday
    /// (`toMonday`), and read as a `Date32` again. A timestamp with time zone is read in the
    /// query time zone, the zone passed to every call, truncated there, and converted back to
    /// UTC: a grain of a day or more gives the local midnight of the first day of the period.
    pub(super) fn date_trunc(
        &mut self,
        unit: DateUnit,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        let operand_type = self.type_of(operand)?;
        let value = self.operand(operand)?;
        match operand_type {
            LogicalType::Date => Ok(truncated_date(unit, &value)),
            LogicalType::Timestamp {
                precision,
                with_time_zone: true,
            } => {
                let zone = self.time_zone();
                let start = match day_truncation(unit) {
                    Some(function) => {
                        format!(
                            "toDateTime64({function}({value}, {zone}), {}, {zone})",
                            digits(precision)
                        )
                    }
                    None => format!("{}({value}, {zone})", time_truncation(unit)),
                };
                Ok(format!("toTimeZone({start}, 'UTC')"))
            }
            _ => Err(self.error(InvariantViolation::UntypedOperand)),
        }
    }

    /// The query time zone, a `text` parameter: `{pn:String}`. In a statement without query
    /// time settings, an integrity check, it is the `time_zone` slot the pipeline binds
    /// (CHK-7.3). `ClickHouse` substitutes a parameter before it analyzes the statement, so the
    /// zone is the constant its time-zone arguments require.
    pub(crate) fn time_zone(&mut self) -> String {
        let Some(time) = self.environment.time else {
            return self.parameters.bind_slot(Binding::TimeZone);
        };
        let name = time.time_zone.as_str().to_owned();
        self.parameters.bind_shared(
            SharedValue::TimeZone,
            LogicalType::Text,
            serde_json::Value::String(name),
            "String",
        )
    }

    /// The logical type of a literal, as E0.f types it (E0F-9.3).
    pub(super) fn literal_type(&self, literal: &Literal) -> Result<LogicalType, GenerateError> {
        self.type_of(&Expression::Literal(literal.clone()))
    }

    /// The logical type of `expression`, or `None` when it cannot be typed.
    pub(super) fn known_type(&self, expression: &Expression) -> Option<LogicalType> {
        self.type_of(expression).ok()
    }

    /// The logical type of `expression`, typed in the scope of the input columns and the
    /// declared attributes.
    pub(crate) fn type_of(&self, expression: &Expression) -> Result<LogicalType, GenerateError> {
        let resolve = |reference: &Reference| self.column_type(reference);
        let attributes = |name: &str| -> Outcome<Typed> {
            let declared = self
                .declared_type(name)
                .map_err(|_| Stop::ReportedElsewhere)?;
            Ok(Typed::value(attribute_logical_type(declared), false))
        };
        let scope =
            TypingScope::new(&resolve, Aggregates::Allowed).with_user_attributes(&attributes);
        type_expression(expression, &scope)
            .ok()
            .and_then(|typed| typed.logical().cloned())
            .ok_or_else(|| self.error(InvariantViolation::UntypedOperand))
    }

    /// The type of an input column, or of a catalog column as the catalog declares it (CAT-4).
    fn column_type(&self, reference: &Reference) -> Outcome<Typed> {
        if reference.qualifier.is_some() {
            return Err(Stop::ReportedElsewhere);
        }
        let found = match self.columns {
            Columns::Input(schema) => schema
                .columns()
                .iter()
                .find(|column| column.name.as_str() == reference.name.text())
                .map(|column| Typed::value(column.logical_type.clone(), column.nullable)),
            Columns::Catalog(relation) => catalog_column(relation, &reference.name)
                .map(|column| Typed::value(column.logical_type.clone(), column.nullable)),
        };
        found.ok_or(Stop::ReportedElsewhere)
    }
}

/// A date truncated to `unit`, as a `Date32`: a unit of a day or less leaves it as it is.
fn truncated_date(unit: DateUnit, value: &str) -> String {
    match day_truncation(unit) {
        Some(function) if unit != DateUnit::Day => format!("toDate32({function}({value}))"),
        _ => value.to_owned(),
    }
}

/// The `ClickHouse` function that gives the first day of the period of `unit`, a unit of a
/// day or more, as a date (E6-7.6): `toMonday` for ISO weeks.
pub(crate) fn day_truncation(unit: DateUnit) -> Option<&'static str> {
    match unit {
        DateUnit::Year => Some("toStartOfYear"),
        DateUnit::Quarter => Some("toStartOfQuarter"),
        DateUnit::Month => Some("toStartOfMonth"),
        DateUnit::Week => Some("toMonday"),
        DateUnit::Day => Some("toDate32"),
        DateUnit::Hour | DateUnit::Minute | DateUnit::Second => None,
    }
}

/// The `ClickHouse` function that truncates a timestamp to a unit of less than a day.
fn time_truncation(unit: DateUnit) -> &'static str {
    match unit {
        DateUnit::Hour => "toStartOfHour",
        DateUnit::Minute => "toStartOfMinute",
        _ => "toStartOfSecond",
    }
}
