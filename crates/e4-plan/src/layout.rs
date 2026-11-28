//! Where every field of a branch is read: the home `Scan` or the `Scan` of a join instance,
//! and the name of its plan column (E3-4.2a, E6-4.12).

use std::collections::BTreeSet;

use menura_model::internal_model::{
    Dataset, FieldPath, InternalModel, NormalizedName, OpenEnd, ValidToBound,
};
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
    /// Along a temporal relationship, the coverage its `Join` also requires, over the plan
    /// columns (VAL-2.8): `from_time` read from the instance the step starts from (E3-4.2a),
    /// the range read from this instance. `None` for a plain relationship.
    pub(crate) temporal: Option<PlacedCoverage>,
}

/// The columns of the coverage condition of a temporal `Join` (VAL-2.7, VAL-2.8).
pub(crate) struct PlacedCoverage {
    /// The column of `from_time`, on the left side.
    pub(crate) from_time: ColumnName,
    /// The column of `valid_from`, on the right side.
    pub(crate) valid_from: ColumnName,
    /// The column of `valid_to`, on the right side.
    pub(crate) valid_to: ColumnName,
    /// Whether `valid_to` is inclusive or exclusive (VAL-1.1).
    pub(crate) valid_to_bound: ValidToBound,
    /// The `valid_to` value that reads like NULL, when declared (VAL-1.2).
    pub(crate) open_end: Option<OpenEnd>,
}

/// The datasets and column names of one branch.
pub(crate) struct Layout<'model> {
    home: &'model Dataset,
    branch: &'model Branch,
    instances: Vec<PlacedInstance<'model>>,
    identity: ColumnName,
    /// The columns of the home `Scan` and of the `Scan` of every instance, and the identity.
    taken: BTreeSet<String>,
}

impl<'model> Layout<'model> {
    /// The layout of `branch`, whose home dataset is `home`, over `model`.
    ///
    /// # Errors
    ///
    /// An internal error for an instance whose dataset or relationship the model does not
    /// have or does not match its step, whose dataset has row policies, which need
    /// visibility on every `Scan` (E4-6.5c), or whose relationship columns are neither fields
    /// nor source columns of its datasets (E0D-3.1).
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
            taken: BTreeSet::new(),
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
        taken.insert(identity);
        layout.taken = taken;
        Ok(layout)
    }

    /// The name of a column that the plan synthesizes from `base`, which starts with the
    /// `menura.` prefix: `base` followed by as many `.` as make it differ from every column of
    /// the branch (E6-4.12). A source column never collides with it: its prefix is
    /// `menura.source.`.
    pub(crate) fn synthesized(&self, base: &str) -> ColumnName {
        let mut name = String::from(base);
        while self.taken.contains(&name) {
            name.push('.');
        }
        ColumnName::new(&name)
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
        let names = instance_names(dataset, model);
        let prefix = self.unique_prefix(instance, &names, taken);
        taken.extend(
            names
                .iter()
                .map(|name| prefix.column(name.as_str()).as_str().to_owned()),
        );
        let mut on = Vec::new();
        for (from, to) in relationship
            .from_columns
            .iter()
            .zip(&relationship.to_columns)
        {
            let left = self
                .key_column(instance.parent, parent_dataset, from)
                .map_err(fail)?;
            let right = prefix.column(key_name(dataset, to).as_str());
            on.push(JoinCondition { left, right });
        }
        let temporal = match model.temporal_range(relationship) {
            None if relationship.temporal.is_some() => {
                return Err(fail(InvariantViolation::UnknownRelationship));
            }
            None => None,
            Some(range) => Some(PlacedCoverage {
                from_time: self
                    .field_column(instance.parent, parent_dataset, range.from_time)
                    .map_err(fail)?,
                valid_from: field_in(dataset, &prefix, range.valid_from).map_err(fail)?,
                valid_to: field_in(dataset, &prefix, range.valid_to).map_err(fail)?,
                valid_to_bound: range.valid_to_bound,
                open_end: range.open_end.cloned(),
            }),
        };
        Ok(PlacedInstance {
            dataset,
            origin,
            relationship: step.relationship.clone(),
            prefix,
            on,
            temporal,
        })
    }

    /// The relationship names along the path of `instance`, each followed by `.`, then as
    /// many more `.` as needed so that no column of `names` under it is `taken` and none
    /// starts with the source prefix of the home dataset (E6-4.12).
    fn unique_prefix(
        &self,
        instance: &JoinInstance,
        names: &[NormalizedName],
        taken: &BTreeSet<String>,
    ) -> ColumnPrefix {
        let mut text = String::new();
        let mut current = Some(instance);
        let mut path = Vec::new();
        while let Some(found) = current {
            path.push(found.step.relationship.as_str());
            current = found
                .parent
                .and_then(|parent| self.branch.join_tree.instance(parent));
        }
        for name in path.into_iter().rev() {
            text.push_str(name);
            text.push('.');
        }
        let source = source_prefix(self.home);
        loop {
            let collides = names.iter().any(|name| {
                let column = format!("{text}{}", name.as_str());
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

    /// The plan column of the relationship key column `key` of `dataset`, the dataset of
    /// `instance`: the column of the field of that name, or else of the source column of that
    /// name (E0D-3.1).
    fn key_column(
        &self,
        instance: Option<InstanceId>,
        dataset: &Dataset,
        key: &NormalizedName,
    ) -> Result<ColumnName, InvariantViolation> {
        if dataset.field(key).is_some() {
            return self.field_column(instance, dataset, key);
        }
        let name = source_key(dataset, key);
        match instance.and_then(|id| self.instances.get(id.index())) {
            None if instance.is_none() => Ok(ColumnName::new(name.as_str())),
            None => Err(InvariantViolation::UnknownDataset),
            Some(placed) => Ok(placed.prefix.column(name.as_str())),
        }
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

/// The column of the field `field` of `dataset` under `prefix`, the prefix of an instance of
/// `dataset`.
fn field_in(
    dataset: &Dataset,
    prefix: &ColumnPrefix,
    field: &NormalizedName,
) -> Result<ColumnName, InvariantViolation> {
    dataset
        .field(field)
        .map(|_| prefix.column(field.as_str()))
        .ok_or(InvariantViolation::ForeignField)
}

/// The name, under the prefix of an instance of `dataset`, of the relationship key column
/// `key` of `dataset`: the field name when `dataset` has that field, or else the
/// [`source_key`] name (E0D-3.1).
fn key_name(dataset: &Dataset, key: &NormalizedName) -> NormalizedName {
    if dataset.field(key).is_some() {
        key.clone()
    } else {
        source_key(dataset, key)
    }
}

/// The name of the source column `key` of `dataset` in a `Scan` of `dataset`: under the
/// [`source_prefix`] of `dataset`, as a dataset metric reads it (E0D-3.1, E6-4.12).
fn source_key(dataset: &Dataset, key: &NormalizedName) -> NormalizedName {
    NormalizedName::new(&format!("{}{}", source_prefix(dataset), key.as_str()))
}

/// The names an instance of `dataset` can project under its prefix: its fields, then the
/// [`source_key`] names of the relationship key columns of `dataset` that are not fields.
fn instance_names(dataset: &Dataset, model: &InternalModel) -> Vec<NormalizedName> {
    let mut names: Vec<NormalizedName> = dataset
        .fields
        .iter()
        .map(|field| field.name.clone())
        .collect();
    for relationship in &model.parts().relationships {
        let mut keys = Vec::new();
        if relationship.from == dataset.name {
            keys.extend(&relationship.from_columns);
        }
        if relationship.to == dataset.name {
            keys.extend(&relationship.to_columns);
        }
        for key in keys {
            let name = key_name(dataset, key);
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}
