//! The `Population` node (E4-3.12): the expected (entity, period) pairs of a population
//! (POP-3) over the visible rows V(D) of the dataset that declares it (E4-6.5, E2-4.9).

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::Grain;
use menura_model::internal_model::{
    Dataset, InternalModel, NormalizedName, Population, PopulationMode,
};
use menura_representations::logical_plan::{
    Membership, Operation, OutputSchema, PlanNode, PopulationRule, SchemaColumn,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use menura_security::AdmittedSecurityContext;

use crate::columns::{over_scan, read_fields};
use crate::error::{InvariantViolation, PlanError};
use crate::followed::PolicyGraph;
use crate::policy_visits::row_filters_from;
use crate::scan::scan;
use crate::visibility::Visibility;

/// What the expected pairs of one population are planned from.
#[derive(Debug, Clone)]
pub struct PopulationRequest<'request> {
    /// The home dataset of the branch whose metric uses the population: its row policies are
    /// visited first (E4-6.9).
    pub home: &'request NormalizedName,
    /// The population, by normalized name.
    pub population: &'request NormalizedName,
    /// The grain of the periods: the `snapshot_grain` of the metric that uses it (POP-3.1).
    pub grain: Grain,
    /// The query's unit of periods (ADD-4.7): one row per period, each the start of a period
    /// at `grain`.
    pub periods: PlanNode,
    /// The column of `periods` that holds the period.
    pub period: ColumnName,
    /// Whether the query has a `reference_time` (E4-6.9b).
    pub has_reference_time: bool,
}

/// Plans the `Population` node of `request`: the expected (entity, period) pairs of its
/// population over its unit of periods, read from the visible rows of the dataset that
/// declares it for the user of `security` (E4-3.12, E4-6.5, E2-4.9).
///
/// # Errors
///
/// [`PlanError::MissingAttribute`] (`ESEC-02`) when an applicable row policy needs an
/// attribute `security` does not provide, visiting the policies of the visible rows of the
/// home dataset, then those of the population dataset, then those of its own visible rows,
/// each dataset once (E4-6.9); then
/// [`PlanError::MissingReferenceTime`] (`EQRY-03`) when one reads the current date or time
/// and the query has no `reference_time`, in the same order (E4-6.9b);
/// [`PlanError::Invariant`] for a population or home dataset the model does not have, a
/// dataset E0 marks unusable, or a `period` column that `periods` does not output.
pub fn plan_population(
    request: PopulationRequest<'_>,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, PlanError> {
    let (dataset, population) = declared(model, request.population)?;
    let origin = Origin::Model(population.location.clone());
    let fail = |violation| PlanError::invariant(violation, &origin);
    if !dataset.query_errors.is_empty() {
        return Err(fail(InvariantViolation::UnusableDataset));
    }
    let rule = population_rule(population).map_err(fail)?;
    let visible = visible_rows(
        &request, dataset, population, &rule, security, model, catalog,
    )?;
    let schema = output_schema(&visible, &rule, &request.periods, &request.period).map_err(fail)?;
    let PopulationRequest {
        grain,
        periods,
        period,
        ..
    } = request;
    Ok(PlanNode::new(
        Operation::Population {
            input: Box::new(visible),
            dataset: dataset.name.clone(),
            population: population.name.clone(),
            grain,
            periods: Box::new(periods),
            period,
            rule,
        },
        schema,
        origin,
    ))
}

/// The visible rows V(D) of `dataset` that `population` reads: its `Scan` of the population
/// fields, the fields of `required_when` and the fields its visibility needs, under its
/// row-policy `Filter` and `SemiJoin` nodes (E4-6.5, E2-4.9). The row filters are computed in
/// the E4-6.9 visit order: the home dataset of `request` first, then `dataset`.
fn visible_rows(
    request: &PopulationRequest<'_>,
    dataset: &Dataset,
    population: &Population,
    rule: &PopulationRule,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, PlanError> {
    let home = model.dataset(request.home).ok_or_else(|| {
        PlanError::invariant(
            InvariantViolation::UnknownDataset,
            &Origin::Model(population.location.clone()),
        )
    })?;
    let graph = PolicyGraph::new(model);
    let filters = row_filters_from(
        &[home, dataset],
        model,
        &graph,
        security,
        request.has_reference_time,
    )?;
    let visibility = Visibility {
        model,
        graph: &graph,
        filters: &filters,
        catalog,
    };
    let mut read: Vec<NormalizedName> = population.named_fields().into_iter().cloned().collect();
    if let Some(required_when) = &rule.required_when {
        read_fields(required_when, &mut read);
    }
    read.extend(visibility.fields(dataset)?);
    visibility.visible_rows(scan(dataset, &read, catalog, None)?, dataset, None)
}

/// The dataset that declares the population `name`, and the population.
pub(crate) fn declared<'model>(
    model: &'model InternalModel,
    name: &NormalizedName,
) -> Result<(&'model Dataset, &'model Population), PlanError> {
    model
        .parts()
        .datasets
        .iter()
        .find_map(|dataset| {
            dataset
                .populations
                .iter()
                .find(|population| &population.name == name)
                .map(|population| (dataset, population))
        })
        .ok_or_else(|| {
            PlanError::invariant(
                InvariantViolation::UnknownPopulation,
                &crate::slice::query_root(),
            )
        })
}

/// The rule of `population` over the columns of its dataset's `Scan`, which names each field
/// after its normalized name (E6-4.12).
fn population_rule(population: &Population) -> Result<PopulationRule, InvariantViolation> {
    let column = |field: &NormalizedName| ColumnName::new(field.as_str());
    let membership = match &population.mode {
        PopulationMode::Validity {
            valid_from,
            valid_to,
            partial_period,
        } => Membership::Validity {
            valid_from: column(valid_from),
            valid_to: column(valid_to),
            partial_period: *partial_period,
        },
        PopulationMode::Periodic { period, .. } => Membership::Periodic {
            period: column(period),
        },
    };
    let required_when = population
        .required_when
        .as_ref()
        .map(|condition| over_scan(condition, None))
        .transpose()?;
    Ok(PopulationRule {
        entity: population.entity.iter().map(column).collect(),
        membership,
        required_when,
    })
}

/// The output of the node: the entity columns, as `visible` types them, then `period`, as
/// `periods` types it.
fn output_schema(
    visible: &PlanNode,
    rule: &PopulationRule,
    periods: &PlanNode,
    period: &ColumnName,
) -> Result<OutputSchema, InvariantViolation> {
    let find = |node: &PlanNode, name: &ColumnName| -> Option<SchemaColumn> {
        node.schema()
            .columns()
            .iter()
            .find(|column| &column.name == name)
            .cloned()
    };
    let mut columns = rule
        .entity
        .iter()
        .map(|entity| find(visible, entity).ok_or(InvariantViolation::ForeignField))
        .collect::<Result<Vec<_>, _>>()?;
    columns.push(find(periods, period).ok_or(InvariantViolation::UnknownPeriodColumn)?);
    Ok(OutputSchema::new(columns))
}
