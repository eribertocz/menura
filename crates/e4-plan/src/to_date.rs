//! `period_to_date` (E4-8.5, E4-8.6): the time key of its context query, checked for
//! `E4-02`, and the `ToDate` node of each of its branches (E4-3.15).

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::Expression;
use menura_model::internal_model::{FieldPath, InternalModel, ValueType};
use menura_representations::bound_query::{BoundDimensionKind, BoundQuery};
use menura_representations::logical_plan::{
    NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn, ToDateGrains, ToDateRange,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use menura_representations::paths::QueryElement;

use crate::columns::column_reference;
use crate::dimensions::grain_unit;
use crate::error::InvariantViolation;
use crate::layout::Layout;

/// The base of the synthesized column that holds the period t of a `ToDate` (E6-4.12).
const PERIOD: &str = "menura.period";

/// How the branches of a `period_to_date` context query repeat their rows (E4-8.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToDatePlan {
    /// The position of the time key among the query's dimensions (E4-8.6).
    position: usize,
    /// Its field, the `time_dimension` of the modifier.
    field: FieldPath,
    /// The query grain and the `grain` of the modifier.
    grains: ToDateGrains,
    /// The query `date_range`, when it is on `time_dimension`.
    range: Option<ToDateRange>,
}

/// The time key of a `period_to_date` over `time_dimension` at `grain` in `query`, or `None`
/// for `E4-02` (E4-8.6, CTX-3.1): exactly one query dimension must have that field, at its `time_grains`
/// grain, or at grain `day` for a `date` without one; a `timestamp` needs a time grain; the
/// query grain must not be coarser than `grain`; and weeks do not nest in a month or a quarter.
pub(crate) fn to_date_plan(
    query: &BoundQuery,
    model: &InternalModel,
    time_dimension: &FieldPath,
    grain: Grain,
) -> Option<ToDatePlan> {
    let mut matching = query.columns().dimensions().enumerate().filter_map(
        |(position, dimension)| match &dimension.kind {
            BoundDimensionKind::Field {
                field, time_grain, ..
            } if field == time_dimension => Some((position, *time_grain)),
            _ => None,
        },
    );
    let (Some((position, time_grain)), None) = (matching.next(), matching.next()) else {
        return None;
    };
    let query_grain = match (time_grain, field_type(model, time_dimension)) {
        (Some(time_grain), _) => time_grain,
        (None, Some(LogicalType::Date)) => Grain::Day,
        (None, _) => return None,
    };
    let nests = rank(query_grain) <= rank(grain)
        && !(query_grain == Grain::Week && matches!(grain, Grain::Month | Grain::Quarter));
    if !nests {
        return None;
    }
    let range = query
        .date_range()
        .filter(|range| &range.field == time_dimension)
        .map(|range| ToDateRange {
            from: range.from,
            to: range.to,
        });
    Some(ToDatePlan {
        position,
        field: time_dimension.clone(),
        grains: ToDateGrains {
            query: query_grain,
            to_date: grain,
        },
        range,
    })
}

/// The `ToDate` of one branch: the node over the rows of the branch, and the keys of its
/// `Deduplicate` and `Aggregate`, whose time key reads the period t (E4-8.5, E4-4.0f).
pub(crate) struct ToDateStep {
    /// The column of the time value.
    dimension: ColumnName,
    /// The column of t.
    period: ColumnName,
    /// The grains.
    grains: ToDateGrains,
    /// The kept range.
    range: Option<ToDateRange>,
    /// The branch keys, the time key reading t.
    pub(crate) keys: Vec<NamedExpression>,
}

impl ToDatePlan {
    /// The step of the branch of `layout` whose keys are `keys`.
    ///
    /// # Errors
    ///
    /// An internal error when the time key reads a field outside its instance.
    pub(crate) fn step(
        &self,
        layout: &Layout<'_>,
        keys: &[NamedExpression],
    ) -> Result<ToDateStep, InvariantViolation> {
        let element = QueryElement::Dimension(self.position);
        let dimension = layout.element_column(&element, &self.field)?;
        let period = layout.synthesized(PERIOD);
        let mut keys = keys.to_vec();
        let key = keys
            .get_mut(self.position)
            .ok_or(InvariantViolation::ForeignField)?;
        let column = column_reference(period.as_str())?;
        key.expression = match key.expression {
            Expression::DateTrunc { .. } => Expression::DateTrunc {
                unit: grain_unit(self.grains.query),
                operand: Box::new(column),
            },
            _ => column,
        };
        Ok(ToDateStep {
            dimension,
            period,
            grains: self.grains,
            range: self.range,
            keys,
        })
    }
}

impl ToDateStep {
    /// The `ToDate` of `input`, whose origin is `origin`: its output is the columns of `input`,
    /// then t, of the type of the time value and never NULL (E4-3.15).
    ///
    /// # Errors
    ///
    /// [`InvariantViolation::ForeignField`] when `input` lacks the time value.
    pub(crate) fn node(
        &self,
        input: PlanNode,
        origin: Origin,
    ) -> Result<PlanNode, InvariantViolation> {
        let time = input
            .schema()
            .columns()
            .iter()
            .find(|column| column.name == self.dimension)
            .ok_or(InvariantViolation::ForeignField)?;
        let mut columns = input.schema().columns().to_vec();
        columns.push(SchemaColumn {
            name: self.period.clone(),
            logical_type: time.logical_type.clone(),
            nullable: false,
        });
        Ok(PlanNode::new(
            Operation::ToDate {
                input: Box::new(input),
                dimension: self.dimension.clone(),
                grains: self.grains,
                range: self.range,
                period: self.period.clone(),
            },
            OutputSchema::new(columns),
            origin,
        ))
    }
}

/// The logical type of `field`, when E0.f typed it.
fn field_type(model: &InternalModel, field: &FieldPath) -> Option<LogicalType> {
    let typed = model
        .dataset(&field.dataset)?
        .field(&field.field)?
        .typed
        .as_ref()?;
    match &typed.value_type {
        ValueType::Logical(logical) => Some(logical.clone()),
        ValueType::Null => None,
    }
}

/// The order of grains, each finer than the next (E4-8.6).
fn rank(grain: Grain) -> u8 {
    match grain {
        Grain::Day => 0,
        Grain::Week => 1,
        Grain::Month => 2,
        Grain::Quarter => 3,
        Grain::Year => 4,
    }
}
