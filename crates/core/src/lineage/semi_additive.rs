//! The lineage of the `Population` and `Snapshot` nodes of semi-additive metrics (E4-3.12,
//! E4-3.13): their values include the population and carry-forward reads (E2-4.9, PLN-4.5).

use menura_model::expression::Expression;
use menura_representations::logical_plan::{
    Membership, Operation, PlanNode, PopulationRule, SnapshotOver, SnapshotRule,
};
use menura_representations::names::ColumnName;

use super::provenance::{NodeLineage, Provenance};
use super::references::{columns_read, component_references};
use super::scan::Sources;
use super::walk::read_by;
use crate::compile::InternalError;

impl Sources<'_> {
    /// The lineage of the `Population` or `Snapshot` `node`.
    ///
    /// # Errors
    ///
    /// An [`InternalError`] for an element the plan names and its inputs do not have.
    pub(super) fn semi_additive(&self, node: &PlanNode) -> Result<NodeLineage, InternalError> {
        match node.operation() {
            Operation::Population {
                input,
                periods,
                rule,
                ..
            } => {
                let mut lineage = self.node(input, None)?;
                let read = population_reads(&lineage, rule);
                lineage.absorb_reads(self.node(periods, None)?);
                lineage.columns = same_for_every_column(node, &read);
                Ok(lineage)
            }
            Operation::Snapshot { input, rule } => self.snapshot(node, input, rule),
            _ => Err(InternalError::new(
                "core",
                "the lineage of a semi-additive node was asked for another node",
            )),
        }
    }

    /// `Snapshot`: each key and entity column derives from its expression over the rows of
    /// `input` and of the carried or added rows; each component column also from the period,
    /// the entity, its call and filter over those rows, and every column of the expected
    /// entities.
    fn snapshot(
        &self,
        node: &PlanNode,
        input: &PlanNode,
        rule: &SnapshotRule,
    ) -> Result<NodeLineage, InternalError> {
        let mut rows = vec![self.node(input, None)?];
        let mut expected = Provenance::default();
        if let SnapshotOver::Time(time) = &rule.over {
            let population = self.node(&time.expected, None)?;
            for provenance in population.columns.values() {
                expected.absorb(provenance);
            }
            let mut absorbed = population;
            absorbed.columns.clear();
            rows[0].absorb_reads(absorbed);
            for other in rule.inputs().into_iter().skip(1) {
                rows.push(self.node(other, None)?);
            }
        }
        let over: Vec<&Expression> = match &rule.over {
            SnapshotOver::Time(time) => vec![&time.period],
            SnapshotOver::Field(expression) => vec![expression],
        };
        let entity: Vec<&Expression> = rule.entity.iter().map(|named| &named.expression).collect();
        let mut value = expected;
        for lineage in &rows {
            value.absorb(&read_by(
                lineage,
                over.iter().copied().chain(entity.iter().copied()),
            ));
            for component in &rule.components {
                let names = component_references(component)
                    .into_iter()
                    .map(|reference| reference.name.text());
                value.absorb(&lineage.provenance_of(names));
            }
        }
        let mut columns = super::provenance::ColumnProvenance::new();
        for named in rule.keys.iter().chain(&rule.entity) {
            let mut provenance = Provenance::default();
            for lineage in &rows {
                provenance.absorb(&lineage.provenance_of(columns_read(&named.expression)));
            }
            columns.insert(named.name.as_str().to_owned(), provenance);
        }
        for component in &rule.components {
            columns.insert(component.name.as_str().to_owned(), value.clone());
        }
        let mut lineage = NodeLineage::default();
        for rows in rows {
            lineage.absorb_reads(rows);
        }
        lineage.columns = columns;
        check_columns(node, &lineage)?;
        Ok(lineage)
    }
}

/// The provenance of what `rule` reads over the rows of `input`: its entity, its mode's
/// columns and its `required_when` (POP-3).
fn population_reads(input: &NodeLineage, rule: &PopulationRule) -> Provenance {
    let mut names: Vec<&str> = rule.entity.iter().map(ColumnName::as_str).collect();
    match &rule.membership {
        Membership::Validity {
            valid_from,
            valid_to,
            ..
        } => names.extend([valid_from.as_str(), valid_to.as_str()]),
        Membership::Periodic { period } => names.push(period.as_str()),
    }
    let mut provenance = input.provenance_of(names);
    if let Some(condition) = &rule.required_when {
        provenance.absorb(&input.provenance_of(columns_read(condition)));
    }
    provenance
}

/// The provenance `provenance` for every column of `node`.
fn same_for_every_column(
    node: &PlanNode,
    provenance: &Provenance,
) -> super::provenance::ColumnProvenance {
    node.schema()
        .columns()
        .iter()
        .map(|column| (column.name.as_str().to_owned(), provenance.clone()))
        .collect()
}

/// Fails unless `lineage` has a provenance for every column of `node`.
fn check_columns(node: &PlanNode, lineage: &NodeLineage) -> Result<(), InternalError> {
    match node
        .schema()
        .columns()
        .iter()
        .find(|column| !lineage.columns.contains_key(column.name.as_str()))
    {
        Some(missing) => Err(InternalError::new(
            "core",
            format!(
                "the plan outputs the column `{}`, which no input derives",
                missing.name.as_str()
            ),
        )),
        None => Ok(()),
    }
}
