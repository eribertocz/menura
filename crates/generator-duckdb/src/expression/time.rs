//! Time grains (E6-5.2) and the types the generator needs to write them.

use menura_contracts::types::common::LogicalType;
use menura_contracts::types::descriptions::Binding;
use menura_expressions::typing::{Aggregates, Outcome, Stop, Typed, TypingScope, type_expression};
use menura_model::expression::{DateUnit, Expression, Literal, Reference};

use super::placeholders::cast;
use super::{Columns, ExpressionWriter, catalog_column};
use crate::error::{GenerateError, InvariantViolation};
use crate::parameters::SharedValue;
use crate::types::attribute_logical_type;

impl ExpressionWriter<'_, '_> {
    /// `date_trunc('<grain>', x)` in the query time zone (E6-5.2, E6-4.10): a date is cast back
    /// to `DATE`; a timestamp with time zone is converted to the query time zone, truncated and
    /// converted back with `AT TIME ZONE`, the time zone a bound parameter.
    pub(super) fn date_trunc(
        &mut self,
        unit: DateUnit,
        operand: &Expression,
    ) -> Result<String, GenerateError> {
        let operand_type = self.type_of(operand)?;
        let value = self.operand(operand)?;
        let grain = unit_name(unit);
        match operand_type {
            LogicalType::Date => Ok(format!("CAST(date_trunc('{grain}', {value}) AS DATE)")),
            LogicalType::Timestamp {
                with_time_zone: true,
                ..
            } => {
                let zone = self.time_zone();
                Ok(format!(
                    "date_trunc('{grain}', {value} AT TIME ZONE {zone}) AT TIME ZONE {zone}"
                ))
            }
            LogicalType::Timestamp { .. } => Ok(format!("date_trunc('{grain}', {value})")),
            _ => Err(self.error(InvariantViolation::UntypedOperand)),
        }
    }

    /// The query time zone, a `text` parameter: `CAST($tz AS VARCHAR)`. In a statement
    /// without query time settings, an integrity check, it is the `time_zone` slot the
    /// pipeline binds (CHK-7.3). `text` always has a `DuckDB` name, so it is written here
    /// without the fallible lookup.
    pub(crate) fn time_zone(&mut self) -> String {
        let Some(time) = self.environment.time else {
            let placeholder = self.parameters.bind_slot(Binding::TimeZone);
            return cast(&placeholder, "VARCHAR");
        };
        let name = time.time_zone.as_str().to_owned();
        let placeholder = self.parameters.bind_shared(
            SharedValue::TimeZone,
            LogicalType::Text,
            serde_json::Value::String(name),
        );
        cast(&placeholder, "VARCHAR")
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

/// The grain name of `unit`, a literal from a closed set (E6-4.11).
fn unit_name(unit: DateUnit) -> &'static str {
    match unit {
        DateUnit::Year => "year",
        DateUnit::Quarter => "quarter",
        DateUnit::Month => "month",
        DateUnit::Week => "week",
        DateUnit::Day => "day",
        DateUnit::Hour => "hour",
        DateUnit::Minute => "minute",
        DateUnit::Second => "second",
    }
}
