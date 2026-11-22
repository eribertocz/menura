//! The time `over` of a semi-additive branch (ADD-4.2–ADD-4.4, E4-7.3–E4-7.6): its periods
//! with data, its expected entities, and the rows that `missing` reads.

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::{Expression, Literal};
use menura_model::internal_model::{MissingHandling, NormalizedName, SemiAdditive};
use menura_representations::bound_query::BoundDateRange;
use menura_representations::logical_plan::{
    NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn, SnapshotMissing, TimeSnapshot,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::SemiAdditiveBranch;
use super::rule::{over_type, snapshot_fields, unit_key};
use crate::columns::column_reference;
use crate::dimensions::grain_unit;
use crate::error::{InvariantViolation, PlanError};
use crate::nodes::aggregate;
use crate::population::{PopulationRequest, plan_population};
use crate::query_filter::{query_predicate, query_predicate_outside_range};
use crate::rows::{RowsRequest, branch_rows};
use crate::scan::scan;

/// The column of the unit of periods that holds a period.
const PERIOD: &str = "period";

/// The time `over` of `branch`, whose rows are `input` and whose `over` column is `over`, at
/// `grain`, its `snapshot_grain`.
///
/// # Errors
///
/// `ESEC-02` and `EQRY-03` from the row policies of the carried rows and of the population
/// (E4-6.9), and an internal error for a declaration E0 never lets through.
pub(crate) fn time_snapshot(
    branch: &SemiAdditiveBranch<'_>,
    input: &PlanNode,
    over: Expression,
    grain: Grain,
) -> Result<TimeSnapshot, PlanError> {
    let origin = branch.origin();
    let fail = |violation| PlanError::invariant(violation, &origin);
    let declaration = branch.declaration;
    let (Some(population), Some(missing), Some(unexpected)) = (
        &declaration.population,
        declaration.missing,
        declaration.unexpected,
    ) else {
        return Err(fail(InvariantViolation::UnknownPopulation));
    };
    let period = Expression::DateTrunc {
        unit: grain_unit(grain),
        operand: Box::new(over),
    };
    let source = &branch.source;
    let over_type = over_type(declaration, source.home).map_err(fail)?;
    let periods = unit_of_periods(branch, input, &period, grain, over_type)?;
    let expected = plan_population(
        PopulationRequest {
            home: &source.home.name,
            population,
            grain,
            periods,
            period: ColumnName::new(PERIOD),
            has_reference_time: source.has_reference_time,
        },
        source.security,
        source.model,
        source.catalog,
    )?;
    Ok(TimeSnapshot {
        period,
        grain,
        unit: unit_key(branch.query, &declaration.over, over_type, grain),
        missing: snapshot_missing(branch, missing, &expected)?,
        expected: Box::new(expected),
        unexpected,
    })
}

/// The unit of periods of the query (ADD-4.7, E4-7.6): every period of a `date_range` on
/// `over`, else the periods with data of `input`, whose period is `period`.
fn unit_of_periods(
    branch: &SemiAdditiveBranch<'_>,
    input: &PlanNode,
    period: &Expression,
    grain: Grain,
    over_type: &LogicalType,
) -> Result<PlanNode, PlanError> {
    match branch.query.date_range() {
        Some(range) if range.field == branch.declaration.over => {
            Ok(periods_of_range(range, grain, over_type))
        }
        _ => periods_with_data(input.clone(), period.clone(), &branch.origin()),
    }
}

/// The unit of periods of a `date_range` on `over`: every period of `grain` inside it, with
/// data or not (ADD-4.7, E4-3.14), each the start of its period, typed as `over`.
fn periods_of_range(range: &BoundDateRange, grain: Grain, over_type: &LogicalType) -> PlanNode {
    PlanNode::new(
        Operation::Periods {
            from: range.from,
            to: range.to,
            grain,
        },
        OutputSchema::new(vec![SchemaColumn {
            name: ColumnName::new(PERIOD),
            logical_type: over_type.clone(),
            nullable: false,
        }]),
        Origin::Query(range.origin.clone()),
    )
}

/// Without a `date_range` on `over`, the periods with data (E4-7.3): the distinct `period` of the rows of the branch, which
/// passed the row policies and the query filters, for any entity.
fn periods_with_data(
    input: PlanNode,
    period: Expression,
    origin: &Origin,
) -> Result<PlanNode, PlanError> {
    let key = NamedExpression {
        name: ColumnName::new(PERIOD),
        expression: period,
        derived: None,
    };
    aggregate(input, vec![key], Vec::new(), origin)
}

/// The `missing` of the rule, with the rows it reads (E4-7.5, E4-7.6).
fn snapshot_missing(
    branch: &SemiAdditiveBranch<'_>,
    missing: MissingHandling,
    expected: &PlanNode,
) -> Result<SnapshotMissing, PlanError> {
    Ok(match missing {
        MissingHandling::Exclude => SnapshotMissing::Exclude,
        MissingHandling::Error => SnapshotMissing::Error,
        MissingHandling::CarryForward {
            max_carry_forward,
            on_stale,
        } => SnapshotMissing::CarryForward {
            carried: Box::new(carried_rows(branch)?),
            max_carry_forward,
            on_stale,
        },
        MissingHandling::Zero => SnapshotMissing::Zero {
            added: Box::new(added_rows(branch, expected)?),
        },
    })
}

/// The rows carried values are found in: the rows of the branch without the `date_range`
/// predicate on `over` (E4-7.5).
fn carried_rows(branch: &SemiAdditiveBranch<'_>) -> Result<PlanNode, PlanError> {
    let source = &branch.source;
    let predicate =
        query_predicate_outside_range(branch.query, &branch.declaration.over, source.layout)?;
    branch_rows(
        source,
        RowsRequest {
            keys: &branch.keys,
            components: &branch.components,
            fields: &snapshot_fields(branch.declaration),
            predicate,
            replaced_home: None,
        },
    )
}

/// The rows added for `missing: zero` (E4-7.6): each expected (entity, period) pair as a home
/// row, joined like a home row and then filtered by the query filters.
fn added_rows(branch: &SemiAdditiveBranch<'_>, expected: &PlanNode) -> Result<PlanNode, PlanError> {
    let source = &branch.source;
    let origin = branch.origin();
    let replaced = |home_reads: &[NormalizedName]| {
        let home_scan = scan(source.home, home_reads, source.catalog, None)?;
        expected_as_home_rows(expected, branch.declaration, home_scan.schema(), &origin)
            .map_err(|violation| PlanError::invariant(violation, &origin))
    };
    branch_rows(
        source,
        RowsRequest {
            keys: &branch.keys,
            components: &branch.components,
            fields: &snapshot_fields(branch.declaration),
            predicate: query_predicate(branch.query, source.layout)?,
            replaced_home: Some(&replaced),
        },
    )
}

/// The value of the home column `column` in a row added for the expected pair whose entity
/// columns are `entity` and whose period column is `period` (E4-7.6).
fn added_value(
    column: &SchemaColumn,
    declaration: &SemiAdditive,
    period: &SchemaColumn,
    entity: &[SchemaColumn],
) -> Result<Expression, InvariantViolation> {
    let name = NormalizedName::new(column.name.as_str());
    if name == declaration.over.field {
        return column_reference(period.name.as_str());
    }
    match declaration
        .entity
        .iter()
        .position(|field| field.field == name)
    {
        Some(position) => {
            let paired = entity
                .get(position)
                .ok_or(InvariantViolation::ForeignField)?;
            column_reference(paired.name.as_str())
        }
        None => Ok(Expression::Literal(Literal::Null)),
    }
}

/// The `Project` of the expected pairs into home rows with the columns of `home_schema`: each
/// entity field from the population, `over` from the expected period, and NULL for every other
/// home field (E4-7.6).
fn expected_as_home_rows(
    expected: &PlanNode,
    declaration: &SemiAdditive,
    home_schema: &OutputSchema,
    origin: &Origin,
) -> Result<PlanNode, InvariantViolation> {
    let expected_columns = expected.schema().columns();
    let (period, entity) = expected_columns
        .split_last()
        .ok_or(InvariantViolation::UnknownPeriodColumn)?;
    let expressions = home_schema
        .columns()
        .iter()
        .map(|column| {
            Ok(NamedExpression {
                name: column.name.clone(),
                expression: added_value(column, declaration, period, entity)?,
                derived: None,
            })
        })
        .collect::<Result<Vec<_>, InvariantViolation>>()?;
    let columns = home_schema
        .columns()
        .iter()
        .map(|column| SchemaColumn {
            nullable: true,
            ..column.clone()
        })
        .collect();
    Ok(PlanNode::new(
        Operation::Project {
            input: Box::new(expected.clone()),
            expressions,
        },
        OutputSchema::new(columns),
        origin.clone(),
    ))
}
