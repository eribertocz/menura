//! The CAT-8 rules of the catalog state document: CAT-8.1 and CAT-8.4 to CAT-8.7.
//!
//! CAT-8.8 needs the model's summaries and is checked by the caller. CAT-8.9 is a producer
//! guarantee (CAT-7.1.5): a period that is not listed has no rows, so there is nothing to check.

use std::collections::HashMap;

use super::period::canonical_grain;
use super::{RuleViolation, first_repeated};
use crate::types::catalog::{
    AggregateBuild, CatalogSchemaDocument, CatalogStateDocument, CheckResult, Partitions,
    PeriodVersion, RelationData, RelationSchema,
};
use crate::types::common::{Grain, LogicalType};

/// The relations of the schema document in use, by `source`.
type SchemaRelations<'schema> = HashMap<&'schema str, &'schema RelationSchema>;

/// Checks the rules in CAT-8 order; within a rule, in document order.
pub(super) fn check(
    state: &CatalogStateDocument,
    schema: &CatalogSchemaDocument,
) -> Result<(), RuleViolation> {
    let relations: SchemaRelations<'_> = schema
        .relations
        .iter()
        .map(|relation| (relation.source.as_str(), relation))
        .collect();
    unique_sources(state)?;
    sources_in_schema(state, &relations)?;
    partition_columns(state, &relations)?;
    canonical_periods(state)?;
    coverage_order(state)
}

/// CAT-8.1: `source` is unique within the document.
fn unique_sources(state: &CatalogStateDocument) -> Result<(), RuleViolation> {
    let sources = state
        .relations
        .iter()
        .map(|relation| relation.source.as_str());
    match first_repeated(sources) {
        Some(index) => Err(RuleViolation::new(
            format!("/relations/{index}/source"),
            format!(
                "relation `{}` is described more than once",
                state.relations[index].source
            ),
        )),
        None => Ok(()),
    }
}

/// CAT-8.4: every `source` of the state, in `relations` and in `checks`, exists in the schema.
fn sources_in_schema(
    state: &CatalogStateDocument,
    relations: &SchemaRelations<'_>,
) -> Result<(), RuleViolation> {
    let relation_sources = state
        .relations
        .iter()
        .enumerate()
        .map(|(index, relation)| (format!("/relations/{index}/source"), &relation.source));
    let check_sources =
        state
            .checks
            .iter()
            .flatten()
            .enumerate()
            .flat_map(|(check_index, check)| {
                check
                    .verified
                    .iter()
                    .enumerate()
                    .map(move |(index, verified)| {
                        (
                            format!("/checks/{check_index}/verified/{index}/source"),
                            &verified.source,
                        )
                    })
            });
    for (pointer, source) in relation_sources.chain(check_sources) {
        if !relations.contains_key(source.as_str()) {
            return Err(RuleViolation::new(
                pointer,
                format!("relation `{source}` is not in the catalog schema"),
            ));
        }
    }
    Ok(())
}

/// CAT-8.5: `partitions.column` exists in the relation and is of kind `date` or `timestamp`.
fn partition_columns(
    state: &CatalogStateDocument,
    relations: &SchemaRelations<'_>,
) -> Result<(), RuleViolation> {
    for (index, relation) in state.relations.iter().enumerate() {
        let RelationData::Partitions(partitions) = &relation.data else {
            continue;
        };
        let column_type = relations.get(relation.source.as_str()).and_then(|schema| {
            schema
                .columns
                .iter()
                .find(|column| column.name == partitions.column)
                .map(|column| &column.logical_type)
        });
        let problem = match column_type {
            None => "is not a column of the relation",
            Some(LogicalType::Date | LogicalType::Timestamp { .. }) => continue,
            Some(_) => "is not of kind `date` or `timestamp`",
        };
        return Err(RuleViolation::new(
            format!("/relations/{index}/partitions/column"),
            format!(
                "partition column `{}` of relation `{}` {problem}",
                partitions.column, relation.source
            ),
        ));
    }
    Ok(())
}

/// CAT-8.6: every period is in the canonical form of its grain and is not repeated in its list.
fn canonical_periods(state: &CatalogStateDocument) -> Result<(), RuleViolation> {
    for (index, relation) in state.relations.iter().enumerate() {
        let pointer = format!("/relations/{index}");
        match &relation.data {
            RelationData::Version(_) => {}
            RelationData::Partitions(partitions) => partition_periods(&pointer, partitions)?,
            RelationData::AggregateBuild(build) => summary_periods(&pointer, build)?,
        }
    }
    for (index, check) in state.checks.iter().flatten().enumerate() {
        check_periods(index, check)?;
    }
    Ok(())
}

/// The periods of `partitions.versions`, of grain `partitions.grain`.
fn partition_periods(pointer: &str, partitions: &Partitions) -> Result<(), RuleViolation> {
    let list = format!("{pointer}/partitions/versions");
    period_list(&list, &partitions.versions, partitions.grain)
}

/// The periods of a summary, which all have the grain of `coverage.from`.
///
/// Where the document declares no grain, a period has the grain its shape denotes (CAT-8.6);
/// the comparison with the model's summary grain belongs to CAT-8.8.
fn summary_periods(pointer: &str, build: &AggregateBuild) -> Result<(), RuleViolation> {
    let base = format!("{pointer}/aggregate_build");
    let from = &build.coverage.from;
    let grain = canonical_grain(from).ok_or_else(|| {
        RuleViolation::new(
            format!("{base}/coverage/from"),
            format!("period `{from}` is not a canonical period"),
        )
    })?;
    of_grain(format!("{base}/coverage/to"), &build.coverage.to, grain)?;
    period_list(&format!("{base}/built_from"), &build.built_from, grain)?;
    period_list(
        &format!("{base}/integrity_check/verified"),
        &build.integrity_check.verified,
        grain,
    )
}

/// The periods of a list: each of `grain`, none repeated.
fn period_list(list: &str, versions: &[PeriodVersion], grain: Grain) -> Result<(), RuleViolation> {
    for (index, version) in versions.iter().enumerate() {
        of_grain(format!("{list}/{index}/period"), &version.period, grain)?;
    }
    match first_repeated(versions.iter().map(|version| version.period.as_str())) {
        Some(index) => Err(RuleViolation::new(
            format!("{list}/{index}/period"),
            format!(
                "period `{}` is listed more than once",
                versions[index].period
            ),
        )),
        None => Ok(()),
    }
}

fn of_grain(pointer: String, period: &str, grain: Grain) -> Result<(), RuleViolation> {
    if canonical_grain(period) == Some(grain) {
        return Ok(());
    }
    Err(RuleViolation::new(
        pointer,
        format!(
            "period `{period}` is not in the canonical form of grain `{}`",
            grain_name(grain)
        ),
    ))
}

/// The periods a check was run on: each canonical, no relation and period repeated.
fn check_periods(check_index: usize, check: &CheckResult) -> Result<(), RuleViolation> {
    let list = format!("/checks/{check_index}/verified");
    for (index, verified) in check.verified.iter().enumerate() {
        if let Some(period) = &verified.period
            && canonical_grain(period).is_none()
        {
            return Err(RuleViolation::new(
                format!("{list}/{index}/period"),
                format!("period `{period}` is not a canonical period"),
            ));
        }
    }
    let pairs = check
        .verified
        .iter()
        .map(|verified| (verified.source.as_str(), verified.period.as_deref()));
    match first_repeated(pairs) {
        Some(index) => {
            let verified = &check.verified[index];
            let (pointer, period) = match &verified.period {
                Some(period) => (
                    format!("{list}/{index}/period"),
                    format!(" period `{period}`"),
                ),
                None => (format!("{list}/{index}"), String::new()),
            };
            Err(RuleViolation::new(
                pointer,
                format!(
                    "relation `{}`{period} is listed more than once in check `{}`",
                    verified.source, check.target
                ),
            ))
        }
        None => Ok(()),
    }
}

/// CAT-8.7: `coverage.from` ≤ `coverage.to`. Both have one grain (CAT-8.6), so their text
/// order is their chronological order.
fn coverage_order(state: &CatalogStateDocument) -> Result<(), RuleViolation> {
    for (index, relation) in state.relations.iter().enumerate() {
        let RelationData::AggregateBuild(build) = &relation.data else {
            continue;
        };
        let coverage = &build.coverage;
        if coverage.from > coverage.to {
            return Err(RuleViolation::new(
                format!("/relations/{index}/aggregate_build/coverage/to"),
                format!(
                    "coverage of relation `{}` ends at `{}`, before it starts at `{}`",
                    relation.source, coverage.to, coverage.from
                ),
            ));
        }
    }
    Ok(())
}

fn grain_name(grain: Grain) -> &'static str {
    match grain {
        Grain::Day => "day",
        Grain::Week => "week",
        Grain::Month => "month",
        Grain::Quarter => "quarter",
        Grain::Year => "year",
    }
}
