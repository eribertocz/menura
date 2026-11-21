//! Where every field of a branch is read: the home `Scan` or the `Scan` of a join instance,
//! and the name of its plan column (E3-4.2a, E6-4.12).

use std::collections::BTreeSet;

use menura_model::internal_model::{Dataset, FieldPath, InternalModel, NormalizedName};
use menura_representations::logical_plan::JoinCondition;
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;
use menura_representations::paths::{Branch, InstanceId, JoinInstance, QueryElement};

use crate::columns::source_prefix;
use crate::error::{InvariantViolation, PlanError};

/// The name of the synthesized row identity before it is made unique (E4-4.0f).
const IDENTITY: &str = "menura.row_identity";

/// One join instance of the branch, with the dataset it reads and the prefix of its columns.
pub(crate) struct PlacedInstance<'model> {
    /// The dataset the instance reaches.
    pub(crate) dataset: &'model Dataset,
    /// The origin of its `Join`: the relationship it crosses.
    pub(crate) origin: Origin,
    /// The relationship it crosses.
    pub(crate) relationship: NormalizedName,
    /// The prefix of the columns of its `Scan` (E6-4.12).
    pub(crate) prefix: ColumnPrefix,
    /// The equalities of its `Join`, over the plan columns.
    pub(crate) on: Vec<JoinCondition>,
}

/// The datasets and column names of one branch.
pub(crate) struct Layout<'model> {
    home: &'model Dataset,
    branch: &'model Branch,
    instances: Vec<PlacedInstance<'model>>,
    identity: ColumnName,
}

impl<'model> Layout<'model> {
    /// The layout of `branch`, whose home dataset is `home`, over `model`.
    ///
    /// # Errors
    ///
    /// An internal error for an instance whose dataset or relationship the model does not
    /// have or does not match its step, whose dataset has row policies, which need
    /// visibility on every `Scan` (E4-6.5c), or whose relationship columns are not fields.
    pub(crate) fn new(
        branch: &'model Branch,
        home: &'model Dataset,
        model: &'model InternalModel,
    ) -> Result<Self, PlanError> {
        let mut layout = Self {
            home,
            branch,
            instances: Vec::new(),
            identity: ColumnName::new(IDENTITY),
        };
        let mut taken: BTreeSet<String> = home
            .fields
            .iter()
            .map(|field| field.name.as_str().to_owned())
            .collect();
        for instance in branch.join_tree.instances() {
            let placed = layout.place(instance, model, &mut taken)?;
            layout.instances.push(placed);
        }
        let mut identity = String::from(IDENTITY);
        while taken.contains(&identity) {
            identity.push('.');
        }
        layout.identity = ColumnName::new(&identity);
        Ok(layout)
    }

    /// The instance `instance`, placed after the instances before it, with a prefix none of
    /// whose columns is already `taken`; its columns are then taken.
    fn place(
        &self,
        instance: &JoinInstance,
        model: &'model InternalModel,
        taken: &mut BTreeSet<String>,
    ) -> Result<PlacedInstance<'model>, PlanError> {
        let step = &instance.step;
        let home_origin = Origin::Model(self.home.location.clone());
        let relationship = model.relationship(&step.relationship).ok_or_else(|| {
            PlanError::invariant(InvariantViolation::UnknownRelationship, &home_origin)
        })?;
        let origin = Origin::Model(relationship.location.clone());
        let fail = |violation| PlanError::invariant(violation, &origin);
        let dataset = model
            .dataset(&step.to)
            .ok_or_else(|| fail(InvariantViolation::UnknownDataset))?;
        let parent_dataset = self.dataset(instance.parent).map_err(fail)?;
        let matches = relationship.from == step.from
            && relationship.to == step.to
            && parent_dataset.name == step.from
            && relationship.from_columns.len() == relationship.to_columns.len();
        if !matches {
            return Err(fail(InvariantViolation::UnknownRelationship));
        }
        let prefix = self.unique_prefix(instance, dataset, taken);
        taken.extend(
            dataset
                .fields
                .iter()
                .map(|field| prefix.column(field.name.as_str()).as_str().to_owned()),
        );
        let mut on = Vec::new();
        for (from, to) in relationship
            .from_columns
            .iter()
            .zip(&relationship.to_columns)
        {
            let left = self.field_column(instance.parent, parent_dataset, from);
            let right = dataset.field(to).map(|_| prefix.column(to.as_str()));
            match (left, right) {
                (Ok(left), Some(right)) => on.push(JoinCondition { left, right }),
                _ => return Err(fail(InvariantViolation::ForeignField)),
            }
        }
        Ok(PlacedInstance {
            dataset,
            origin,
            relationship: step.relationship.clone(),
            prefix,
            on,
        })
    }

    /// The relationship names along the path of `instance`, each followed by `.`, then as
    /// many more `.` as needed so that no column of `dataset` under it is `taken` and none
    /// starts with the source prefix of the home dataset (E6-4.12).
    fn unique_prefix(
        &self,
        instance: &JoinInstance,
        dataset: &Dataset,
        taken: &BTreeSet<String>,
    ) -> ColumnPrefix {
        let mut text = String::new();
        let mut current = Some(instance);
        let mut names = Vec::new();
        while let Some(found) = current {
            names.push(found.step.relationship.as_str());
            current = found
                .parent
                .and_then(|parent| self.branch.join_tree.instance(parent));
        }
        for name in names.into_iter().rev() {
            text.push_str(name);
            text.push('.');
        }
        let source = source_prefix(self.home);
        loop {
            let collides = dataset.fields.iter().any(|field| {
                let column = format!("{text}{}", field.name.as_str());
                taken.contains(&column) || column.starts_with(&source)
            });
            if !collides {
                return ColumnPrefix::new(&text);
            }
            text.push('.');
        }
    }

    /// The join instances, in path order.
    pub(crate) fn instances(&self) -> &[PlacedInstance<'model>] {
        &self.instances
    }

    /// The name of the synthesized row identity: `menura.row_identity`, followed by as many
    /// `.` as make it differ from every column of the branch (E4-4.0f, E6-4.12).
    pub(crate) fn identity(&self) -> &ColumnName {
        &self.identity
    }

    /// The dataset of the instance `instance`; the home dataset for `None`.
    ///
    /// # Errors
    ///
    /// [`InvariantViolation::UnknownDataset`] for an instance the branch does not have.
    pub(crate) fn dataset(
        &self,
        instance: Option<InstanceId>,
    ) -> Result<&'model Dataset, InvariantViolation> {
        match instance {
            None => Ok(self.home),
            Some(id) => self
                .instances
                .get(id.index())
                .map(|placed| placed.dataset)
                .ok_or(InvariantViolation::UnknownDataset),
        }
    }

    /// The instance `element` is bound to; `None` for the home dataset (E3-4.2a).
    pub(crate) fn bound(&self, element: &QueryElement) -> Option<InstanceId> {
        self.branch.join_tree.bound_instance(element)
    }

    /// The plan column of `field`, read by `element` through the instance it is bound to.
    ///
    /// # Errors
    ///
    /// [`InvariantViolation::ForeignField`] when `field` is not a field of the dataset of
    /// that instance.
    pub(crate) fn element_column(
        &self,
        element: &QueryElement,
        field: &FieldPath,
    ) -> Result<ColumnName, InvariantViolation> {
        let instance = self.bound(element);
        let dataset = self.dataset(instance)?;
        if dataset.name != field.dataset {
            return Err(InvariantViolation::ForeignField);
        }
        self.field_column(instance, dataset, &field.field)
    }

    /// The plan column of the field `field` of `dataset`, the dataset of `instance`.
    fn field_column(
        &self,
        instance: Option<InstanceId>,
        dataset: &Dataset,
        field: &NormalizedName,
    ) -> Result<ColumnName, InvariantViolation> {
        if dataset.field(field).is_none() {
            return Err(InvariantViolation::ForeignField);
        }
        match instance.and_then(|id| self.instances.get(id.index())) {
            None if instance.is_none() => Ok(ColumnName::new(field.as_str())),
            None => Err(InvariantViolation::UnknownDataset),
            Some(placed) => Ok(placed.prefix.column(field.as_str())),
        }
    }
}
