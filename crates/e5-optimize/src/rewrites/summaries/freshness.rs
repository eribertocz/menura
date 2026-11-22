//! Summary freshness (CAT-7.2, SUM-3.7): whether the catalog state proves that a summary holds
//! the rows its base relation would give, and otherwise the first condition that fails.

use std::collections::{BTreeMap, BTreeSet};

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::{
    AggregateBuild, CatalogSchemaDocument, CatalogStateDocument, Column, Partitions, PeriodVersion,
    RelationData,
};
use menura_contracts::types::common::{CheckStatus, Grain, LogicalType, Token};
use menura_model::expression::{Expression, Identifier, Reference};
use menura_model::internal_model::{Dataset, InternalModel, Summary, SummaryDimension, ValueType};
use menura_representations::bound_query::BoundDateRange;
use menura_representations::time::TimeZoneName;

use super::period::{Day, Period};

/// The condition of CAT-7.2 that a summary fails, in the order they are checked. A missing
/// datum fails the first condition that reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailedCondition {
    /// Condition 1: the base has no usable partitions for the summary's coverage dimension.
    Partitions,
    /// Condition 2: a period of P is outside `coverage`.
    Coverage,
    /// Condition 3: `built_from` does not list exactly the base versions of Q.
    BuiltFrom,
    /// Condition 4: the build's `definition_hash` is not the current one.
    DefinitionHash,
    /// Condition 5: the build's time zone is not the query's.
    TimeZone,
    /// Condition 6: the integrity check did not pass on the current definition and versions.
    IntegrityCheck,
}

/// The partitions of the base and the grains of CAT-7.2: g of the partitions, G of the
/// summary's coverage dimension.
struct PartitionEvidence<'a> {
    partitions: &'a Partitions,
    coverage_grain: Grain,
}

/// Checks the conditions of CAT-7.2 for `summary` in the catalog `state`, for a query in
/// `time_zone` whose `date_range`, if any, is resolved to absolute dates (QRY-4.2).
///
/// A `date_range` restricts P only when it is on the base field of the summary's coverage
/// dimension. Returns `Ok(())` when the summary is valid, or else the first condition that
/// fails.
pub(crate) fn check_freshness(
    summary: &Summary,
    model: &InternalModel,
    schema: &CatalogSchemaDocument,
    state: &CatalogStateDocument,
    time_zone: &TimeZoneName,
    date_range: Option<&BoundDateRange>,
) -> Result<(), FailedCondition> {
    let base = model
        .dataset(&summary.of)
        .ok_or(FailedCondition::Partitions)?;
    let coverage_dimension = summary
        .dimensions
        .iter()
        .find(|dimension| dimension.field == summary.coverage_field)
        .ok_or(FailedCondition::Partitions)?;
    let range = date_range
        .filter(|range| {
            range.field.dataset == summary.of && range.field.field == coverage_dimension.base
        })
        .map(|range| (Day::of_date(range.from), Day::of_date(range.to)));
    let evidence = partition_evidence(base, coverage_dimension, schema, state, range.is_some())?;
    let build = model
        .dataset(&summary.dataset)
        .and_then(|dataset| aggregate_build(state, &dataset.source))
        .ok_or(FailedCondition::Coverage)?;
    let periods = periods_to_prove(&evidence, build, range);
    if !inside_coverage(build, evidence.coverage_grain, &periods) {
        return Err(FailedCondition::Coverage);
    }
    if !same_versions(&evidence, &build.built_from, &periods) {
        return Err(FailedCondition::BuiltFrom);
    }
    if build.definition_hash != summary.definition_hash {
        return Err(FailedCondition::DefinitionHash);
    }
    if !time_zone_holds(summary, base, build, time_zone) {
        return Err(FailedCondition::TimeZone);
    }
    let check = &build.integrity_check;
    if check.status != CheckStatus::Passed
        || check.definition_hash != summary.definition_hash
        || !same_versions(&evidence, &check.verified, &periods)
    {
        return Err(FailedCondition::IntegrityCheck);
    }
    Ok(())
}

/// Condition 1: the base's partitions are on the plain column the coverage dimension reads, that
/// column has no time zone, the summary's grain derives from the partitions' grain, every
/// listed period has that grain, and a nullable column comes with a date range on coverage.
fn partition_evidence<'a>(
    base: &Dataset,
    coverage_dimension: &SummaryDimension,
    schema: &CatalogSchemaDocument,
    state: &'a CatalogStateDocument,
    has_range: bool,
) -> Result<PartitionEvidence<'a>, FailedCondition> {
    let failed = FailedCondition::Partitions;
    let Some(RelationData::Partitions(partitions)) = state
        .relations
        .iter()
        .find(|relation| relation.source == base.source)
        .map(|relation| &relation.data)
    else {
        return Err(failed);
    };
    let identifier = base
        .field(&coverage_dimension.base)
        .and_then(|field| plain_column(field.expression.as_ref()))
        .ok_or(failed)?;
    let column = schema_column(schema, &base.source, &partitions.column).ok_or(failed)?;
    let coverage_grain = coverage_dimension.grain.ok_or(failed)?;
    let usable = names_column(identifier, &partitions.column)
        && !has_time_zone(&column.logical_type)
        && (has_range || !column.nullable)
        && derives_from(coverage_grain, partitions.grain)
        && all_of_grain(&partitions.versions, partitions.grain);
    usable
        .then_some(PartitionEvidence {
            partitions,
            coverage_grain,
        })
        .ok_or(failed)
}

/// The identifier of the column a field reads when its expression is a plain column, not an
/// expression.
fn plain_column(expression: Option<&Expression>) -> Option<&Identifier> {
    match expression? {
        Expression::Reference(Reference {
            qualifier: None,
            name,
        }) => Some(name),
        _ => None,
    }
}

/// Whether `identifier` names the catalog column `column`. E0 has already resolved the
/// reference (E0D-1.5) and stores it as the delimited identifier of its normalized text, so the
/// column is the one whose name, normalized as E0D-1.1, equals that text (CAT-8.2 makes it
/// unique).
fn names_column(identifier: &Identifier, column: &str) -> bool {
    normalize_unquoted_identifier(identifier.text()) == normalize_unquoted_identifier(column)
}

/// The column `name` of the relation `source` in the catalog schema.
fn schema_column<'a>(
    schema: &'a CatalogSchemaDocument,
    source: &str,
    name: &str,
) -> Option<&'a Column> {
    schema
        .relations
        .iter()
        .find(|relation| relation.source == source)?
        .columns
        .iter()
        .find(|column| column.name == name)
}

/// Whether `logical_type` is a `timestamp` with time zone.
fn has_time_zone(logical_type: &LogicalType) -> bool {
    matches!(
        logical_type,
        LogicalType::Timestamp {
            with_time_zone: true,
            ..
        }
    )
}

/// Whether `coarse` derives from `fine` by exact grouping (SUM-3.4, CAT-7.2 condition 1).
fn derives_from(coarse: Grain, fine: Grain) -> bool {
    match fine {
        Grain::Day => true,
        Grain::Week => coarse == Grain::Week,
        Grain::Month => matches!(coarse, Grain::Month | Grain::Quarter | Grain::Year),
        Grain::Quarter => matches!(coarse, Grain::Quarter | Grain::Year),
        Grain::Year => coarse == Grain::Year,
    }
}

/// Whether every period of `versions` is a canonical period of `grain`.
fn all_of_grain(versions: &[PeriodVersion], grain: Grain) -> bool {
    versions
        .iter()
        .all(|version| Period::parse(&version.period).is_some_and(|period| period.grain() == grain))
}

/// The summary build of the relation `source`, when the state has one.
fn aggregate_build<'a>(
    state: &'a CatalogStateDocument,
    source: &str,
) -> Option<&'a AggregateBuild> {
    state
        .relations
        .iter()
        .find(|relation| relation.source == source)
        .and_then(|relation| match &relation.data {
            RelationData::AggregateBuild(build) => Some(build),
            RelationData::Version(_) | RelationData::Partitions(_) => None,
        })
}

/// P: the first days of the periods at G that contain a period listed in the base's partitions
/// or in `built_from` and, with a date range on coverage, overlap that range. A range aligned
/// on G (SUM-3.5) overlaps exactly the periods that lie inside it.
fn periods_to_prove(
    evidence: &PartitionEvidence<'_>,
    build: &AggregateBuild,
    range: Option<(Day, Day)>,
) -> BTreeSet<Day> {
    evidence
        .partitions
        .versions
        .iter()
        .chain(&build.built_from)
        .filter_map(|version| Period::parse(&version.period))
        .map(|period| Period::containing(period.first_day(), evidence.coverage_grain))
        .filter(|period| {
            range.is_none_or(|(from, to)| period.first_day() <= to && period.last_day() >= from)
        })
        .map(Period::first_day)
        .collect()
}

/// Condition 2: every period of P is inside `coverage`, whose periods have grain G.
fn inside_coverage(build: &AggregateBuild, coverage_grain: Grain, periods: &BTreeSet<Day>) -> bool {
    let bound = |text: &str| Period::parse(text).filter(|period| period.grain() == coverage_grain);
    let (Some(from), Some(to)) = (bound(&build.coverage.from), bound(&build.coverage.to)) else {
        return false;
    };
    periods
        .iter()
        .all(|first| *first >= from.first_day() && *first <= to.first_day())
}

/// Conditions 3 and 6: every period of `listed` has grain g, and inside Q the base's
/// partitions and `listed` name the same periods with equal versions (CAT-6.1). A period of Q
/// that neither lists needs no evidence (CAT-7.2.1).
fn same_versions(
    evidence: &PartitionEvidence<'_>,
    listed: &[PeriodVersion],
    periods: &BTreeSet<Day>,
) -> bool {
    let grain = evidence.partitions.grain;
    all_of_grain(listed, grain)
        && versions_in_q(
            &evidence.partitions.versions,
            evidence.coverage_grain,
            periods,
        ) == versions_in_q(listed, evidence.coverage_grain, periods)
}

/// The versions of the periods of `versions` that lie inside a period of P, by first day.
fn versions_in_q<'a>(
    versions: &'a [PeriodVersion],
    coverage_grain: Grain,
    periods: &BTreeSet<Day>,
) -> BTreeMap<Day, &'a Token> {
    versions
        .iter()
        .filter_map(|version| {
            let period = Period::parse(&version.period)?;
            let container = Period::containing(period.first_day(), coverage_grain);
            periods
                .contains(&container.first_day())
                .then_some((period.first_day(), &version.version))
        })
        .collect()
}

/// Condition 5: when a time dimension of the summary has a `timestamp` with time zone as its
/// base field, the build's time zone is the query's, compared by code points. A time dimension
/// whose base field has no known type has no evidence that its time zone does not matter.
fn time_zone_holds(
    summary: &Summary,
    base: &Dataset,
    build: &AggregateBuild,
    time_zone: &TimeZoneName,
) -> bool {
    let zone_matters = summary
        .dimensions
        .iter()
        .filter(|dimension| dimension.grain.is_some())
        .any(|dimension| {
            match base
                .field(&dimension.base)
                .and_then(|field| field.typed.as_ref())
            {
                Some(typed) => match &typed.value_type {
                    ValueType::Logical(logical_type) => has_time_zone(logical_type),
                    ValueType::Null => false,
                },
                None => true,
            }
        });
    !zone_matters || build.time_zone == time_zone.as_str()
}
