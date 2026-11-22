//! What a plan column derives from, and what a part of the plan reads besides its values.

use std::collections::{BTreeMap, BTreeSet};

use menura_model::internal_model::{FieldPath, NormalizedName};

/// One catalog column: its relation's `source` and its name as the catalog spells it. Ordered
/// by `source`, then by `column`, comparing code points (PLN-4.5).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CatalogColumn {
    /// The relation's catalog `source`.
    pub(super) source: String,
    /// The column, in the catalog's spelling.
    pub(super) column: String,
}

/// What the value of one plan column derives from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Provenance {
    /// The catalog columns its value reads.
    pub(super) catalog_columns: BTreeSet<CatalogColumn>,
    /// The model fields its value reads, by normalized name.
    pub(super) fields: BTreeSet<FieldPath>,
    /// The relationships of the `Join` nodes that bring a field it reads to a home dataset.
    pub(super) relationships: BTreeSet<NormalizedName>,
    /// The positions, from 0, of the `Stitch` branches it comes from.
    pub(super) branches: BTreeSet<usize>,
}

impl Provenance {
    /// Adds everything `other` derives from.
    pub(super) fn absorb(&mut self, other: &Provenance) {
        self.catalog_columns
            .extend(other.catalog_columns.iter().cloned());
        self.fields.extend(other.fields.iter().cloned());
        self.relationships
            .extend(other.relationships.iter().cloned());
        self.branches.extend(other.branches.iter().copied());
    }
}

/// The provenance of every column of a plan node, by column name.
pub(super) type ColumnProvenance = BTreeMap<String, Provenance>;

/// What a part of the plan reads besides the values of its columns (PLN-4.5, PLN-4.6, PLN-4.7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Reads {
    /// The catalog columns read by its `Filter`, `Join` and `SemiJoin` nodes.
    pub(super) catalog_columns: BTreeSet<CatalogColumn>,
    /// The datasets it scans, a summary counted as its base dataset, by normalized name.
    pub(super) datasets: BTreeSet<NormalizedName>,
    /// The summary it scans instead of its base dataset, if any.
    pub(super) summary: Option<NormalizedName>,
}

impl Reads {
    /// Adds everything `other` reads; the first summary found is kept.
    pub(super) fn absorb(&mut self, other: Reads) {
        self.catalog_columns.extend(other.catalog_columns);
        self.datasets.extend(other.datasets);
        if self.summary.is_none() {
            self.summary = other.summary;
        }
    }
}

/// The lineage of a plan node: the provenance of its columns, what it reads in every branch,
/// and, above a `Stitch`, what each branch reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct NodeLineage {
    /// The provenance of its columns.
    pub(super) columns: ColumnProvenance,
    /// What it reads for every branch.
    pub(super) shared: Reads,
    /// What each `Stitch` branch reads, in branch order; empty without a `Stitch`.
    pub(super) branches: Vec<Reads>,
}

impl NodeLineage {
    /// The provenance of the column `name`, merged from every column `names` lists.
    pub(super) fn provenance_of<'name>(
        &self,
        names: impl IntoIterator<Item = &'name str>,
    ) -> Provenance {
        let mut provenance = Provenance::default();
        for name in names {
            if let Some(found) = self.columns.get(name) {
                provenance.absorb(found);
            }
        }
        provenance
    }

    /// Adds what `other` reads, in every branch and per branch, but not its columns.
    pub(super) fn absorb_reads(&mut self, other: NodeLineage) {
        self.shared.absorb(other.shared);
        for (position, reads) in other.branches.into_iter().enumerate() {
            match self.branches.get_mut(position) {
                Some(existing) => existing.absorb(reads),
                None => self.branches.push(reads),
            }
        }
    }

    /// Adds the catalog columns `provenance` reads to what every branch reads.
    pub(super) fn read_in_every_branch(&mut self, provenance: &Provenance) {
        self.shared
            .catalog_columns
            .extend(provenance.catalog_columns.iter().cloned());
    }
}
