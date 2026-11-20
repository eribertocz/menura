//! The columns of one catalog relation, as references to them find them (E0D-1.5), shared by
//! the substages that look columns up.

use std::collections::{HashMap, HashSet};

use menura_contracts::types::catalog::RelationSchema;
use menura_model::expression::Identifier;

use crate::names::Name;

/// The column names of a catalog relation, each the column's exact spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Columns<'catalog> {
    /// The first column of each name normalized as a regular identifier.
    regular: HashMap<Name, &'catalog str>,
    /// Every exact spelling.
    exact: HashSet<&'catalog str>,
}

impl<'catalog> Columns<'catalog> {
    /// The columns of `relation`.
    pub(crate) fn new(relation: &'catalog RelationSchema) -> Self {
        let mut regular = HashMap::new();
        let mut exact = HashSet::new();
        for column in &relation.columns {
            regular
                .entry(Name::regular(&column.name))
                .or_insert(column.name.as_str());
            exact.insert(column.name.as_str());
        }
        Self { regular, exact }
    }

    /// The exact spelling of the column a reference to `name` matches (E0D-1.5): for a regular
    /// reference, the column whose normalized name is `name`; for a delimited one, the column
    /// spelled exactly as `name`. Catalog admission guarantees at most one column per normalized
    /// name (CAT-8.2), so a regular reference matches at most one column.
    pub(crate) fn find(&self, name: &Name, delimited: bool) -> Option<&'catalog str> {
        if delimited {
            self.exact.get(name.as_str()).copied()
        } else {
            self.regular.get(name).copied()
        }
    }

    /// The column the expression identifier `identifier` matches.
    pub(crate) fn find_identifier(&self, identifier: &Identifier) -> Option<&'catalog str> {
        self.find(&Name::of(identifier), identifier.is_delimited())
    }
}

#[cfg(test)]
mod tests {
    use menura_contracts::admit_catalog_schema;
    use menura_model::expression::Identifier;

    use super::Columns;

    #[test]
    fn e0d_1_5_regular_references_match_normalized_and_delimited_ones_exactly() {
        let catalog = admit_catalog_schema(include_str!(
            "../../../conformance/models/e0-references-catalog-columns/catalog-schema.json"
        ))
        .expect("the catalog schema is admitted");
        let columns = Columns::new(&catalog.relations[0]);
        let regular = |text| columns.find_identifier(&Identifier::regular(text).expect("regular"));
        let delimited =
            |text| columns.find_identifier(&Identifier::delimited(text).expect("delimited"));
        assert_eq!(regular("amount"), Some("Amount"));
        assert_eq!(regular("AMOUNT"), Some("Amount"));
        assert_eq!(delimited("Amount"), Some("Amount"));
        assert_eq!(delimited("amount"), None);
        assert_eq!(delimited("AMOUNT"), None);
        assert_eq!(regular("payment_id"), Some("payment_id"));
        assert_eq!(delimited("PAYMENT_ID"), None);
    }
}
