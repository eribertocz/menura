//! The visible rows V(D) of a dataset over its `Scan` (E4-6.1, E4-6.5, E4-6.5a, E4-6.5b,
//! E4-6.5c): a `Filter` for its own applicable policies, directly over the `Scan`, then one
//! `SemiJoin` per relationship E4-6.5 follows, by relationship name. Each `SemiJoin` holds the
//! unfiltered `Scan` of the related dataset D' and the plan of V(D'), built the same way with
//! D' added to the current chain, so cycles are cut where they come back.

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, GraphEdge, InternalModel, NormalizedName};
use menura_representations::logical_plan::{JoinCondition, Operation, PlanNode};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;

use crate::columns::{column_reference, over_scan, read_fields};
use crate::error::{InvariantViolation, PlanError};
use crate::followed::followed_relationships;
use crate::policy_visits::{RowFilter, RowFilters};
use crate::rewrite::rewrite;
use crate::scan::scan;

/// What the visible rows of every dataset are built from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Visibility<'plan> {
    /// The model.
    pub(crate) model: &'plan InternalModel,
    /// The row filters of the user, by dataset.
    pub(crate) filters: &'plan RowFilters,
    /// The admitted catalog schema.
    pub(crate) catalog: &'plan CatalogSchemaDocument,
}

impl Visibility<'_> {
    /// The fields of `dataset` its visible rows read over its own `Scan`: those of its own row
    /// filter, then the `from_columns` of every relationship it follows.
    ///
    /// # Errors
    ///
    /// An internal error for a relationship the model does not have or an empty name.
    pub(crate) fn fields(&self, dataset: &Dataset) -> Result<Vec<NormalizedName>, PlanError> {
        self.fields_on_chain(dataset, std::slice::from_ref(&dataset.name))
    }

    /// The `Scan` columns of the fields that the visible rows of `dataset`, the dataset of a
    /// join instance whose columns are under `prefix`, read.
    ///
    /// # Errors
    ///
    /// As [`Self::fields`].
    pub(crate) fn instance_fields(
        &self,
        dataset: &Dataset,
        prefix: &ColumnPrefix,
    ) -> Result<Vec<NormalizedName>, PlanError> {
        Ok(self
            .fields(dataset)?
            .iter()
            .map(|field| NormalizedName::new(prefix.column(field.as_str()).as_str()))
            .collect())
    }

    /// The visible rows of `dataset` over its `scan`, whose columns are the fields of
    /// `dataset` under `prefix` (none for the home dataset).
    ///
    /// # Errors
    ///
    /// An internal error for a relationship or dataset the model does not have, a related
    /// dataset E0 marks unusable, or an empty name.
    pub(crate) fn visible_rows(
        &self,
        scan: PlanNode,
        dataset: &Dataset,
        prefix: Option<&ColumnPrefix>,
    ) -> Result<PlanNode, PlanError> {
        self.visible_on_chain(scan, dataset, prefix, std::slice::from_ref(&dataset.name))
    }

    fn fields_on_chain(
        &self,
        dataset: &Dataset,
        chain: &[NormalizedName],
    ) -> Result<Vec<NormalizedName>, PlanError> {
        let mut fields = Vec::new();
        if let Some(own) = self.own_filter(dataset)? {
            let predicate = over_scan(&own.filter, None).map_err(|violation| {
                PlanError::invariant(violation, &Origin::Model(own.location.clone()))
            })?;
            read_fields(&predicate, &mut fields);
        }
        for edge in followed_relationships(self.model, &dataset.name, chain) {
            let relationship = self
                .model
                .relationship(&edge.relationship)
                .ok_or_else(|| unknown_relationship(dataset))?;
            fields.extend(relationship.from_columns.iter().cloned());
        }
        Ok(fields)
    }

    fn visible_on_chain(
        &self,
        scanned: PlanNode,
        dataset: &Dataset,
        prefix: Option<&ColumnPrefix>,
        chain: &[NormalizedName],
    ) -> Result<PlanNode, PlanError> {
        let mut node = scanned;
        if let Some(own) = self.own_filter(dataset)? {
            let origin = Origin::Model(own.location.clone());
            let predicate = over_scan(&own.filter, None)
                .and_then(|predicate| prefixed(&predicate, prefix))
                .map_err(|violation| PlanError::invariant(violation, &origin))?;
            let schema = node.schema().clone();
            node = PlanNode::new(
                Operation::Filter {
                    input: Box::new(node),
                    predicate,
                },
                schema,
                origin,
            );
        }
        for edge in followed_relationships(self.model, &dataset.name, chain) {
            node = self.semi_join(node, dataset, edge, prefix, chain)?;
        }
        Ok(node)
    }

    /// The row filter of `dataset`; `None` when it has no policy. A dataset with policies
    /// whose filter was not computed is an internal error, never a dataset without filter.
    fn own_filter(&self, dataset: &Dataset) -> Result<Option<&RowFilter>, PlanError> {
        match self.filters.of(&dataset.name) {
            Some(own) => Ok(Some(own)),
            None if dataset.row_policies.is_empty() => Ok(None),
            None => Err(PlanError::invariant(
                InvariantViolation::UnvisitedPolicies,
                &Origin::Model(dataset.location.clone()),
            )),
        }
    }

    /// The `SemiJoin` of `input`, rows of `dataset`, along `edge`.
    fn semi_join(
        &self,
        input: PlanNode,
        dataset: &Dataset,
        edge: &GraphEdge,
        prefix: Option<&ColumnPrefix>,
        chain: &[NormalizedName],
    ) -> Result<PlanNode, PlanError> {
        let relationship = self
            .model
            .relationship(&edge.relationship)
            .ok_or_else(|| unknown_relationship(dataset))?;
        let origin = Origin::Model(relationship.location.clone());
        let related = self
            .model
            .dataset(&edge.to)
            .ok_or_else(|| PlanError::invariant(InvariantViolation::UnknownDataset, &origin))?;
        if !related.query_errors.is_empty() {
            return Err(PlanError::invariant(
                InvariantViolation::UnusableDataset,
                &Origin::Model(related.location.clone()),
            ));
        }
        let on = relationship
            .from_columns
            .iter()
            .zip(&relationship.to_columns)
            .map(|(from, to)| JoinCondition {
                left: column_of(from, prefix),
                right: ColumnName::new(to.as_str()),
            })
            .collect();
        let mut related_chain = chain.to_vec();
        related_chain.push(related.name.clone());
        let mut visible_fields = relationship.to_columns.clone();
        visible_fields.extend(self.fields_on_chain(related, &related_chain)?);
        let related_all = scan(related, &relationship.to_columns, self.catalog, None)?;
        let related_visible = self.visible_on_chain(
            scan(related, &visible_fields, self.catalog, None)?,
            related,
            None,
            &related_chain,
        )?;
        let schema = input.schema().clone();
        Ok(PlanNode::new(
            Operation::SemiJoin {
                input: Box::new(input),
                relationship: relationship.name.clone(),
                on,
                related_all: Box::new(related_all),
                related_visible: Box::new(related_visible),
            },
            schema,
            origin,
        ))
    }
}

fn unknown_relationship(dataset: &Dataset) -> PlanError {
    PlanError::invariant(
        InvariantViolation::UnknownRelationship,
        &Origin::Model(dataset.location.clone()),
    )
}

/// The `Scan` column of `field` under `prefix`.
fn column_of(field: &NormalizedName, prefix: Option<&ColumnPrefix>) -> ColumnName {
    prefix.map_or_else(
        || ColumnName::new(field.as_str()),
        |prefix| prefix.column(field.as_str()),
    )
}

/// `predicate`, over the unprefixed `Scan` columns, with every column under `prefix`.
fn prefixed(
    predicate: &Expression,
    prefix: Option<&ColumnPrefix>,
) -> Result<Expression, InvariantViolation> {
    let Some(prefix) = prefix else {
        return Ok(predicate.clone());
    };
    rewrite(predicate, &mut |node| match node {
        Expression::Reference(reference) => column_reference(
            column_of(&NormalizedName::new(reference.name.text()), Some(prefix)).as_str(),
        )
        .map(Some),
        _ => Ok(None),
    })
}
