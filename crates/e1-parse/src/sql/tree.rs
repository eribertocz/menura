//! The query tree of a checked SQL statement (E1-5): columns in `SELECT` order (E1-4.3).

use menura_model::expression::{Identifier, Reference};
use menura_representations::columns::OutputColumn;
use menura_representations::names::OutputName;
use menura_representations::query_tree::{
    QueryDimension, QueryDimensionKind, QueryMetric, QueryMetricKind, QueryReference, ReferencePart,
};

use super::grouping::Items;
use super::items::{ItemKind, output_identifier};
use super::position::{Failure, location, unparsable};

/// The output columns of `items`, in `SELECT` order.
///
/// # Errors
///
/// None in practice: a reference always has its name as a part.
pub(super) fn columns(
    sql: &str,
    items: &Items,
) -> Result<Vec<OutputColumn<QueryDimension, QueryMetric>>, Failure> {
    items
        .iter()
        .map(|checked| {
            let (item, kind) = (&checked.item, &checked.kind);
            let output_name =
                OutputName::new(output_identifier(item, kind).map_or("", Identifier::text));
            let origin = location(sql, item.expression.start);
            let tree = item.expression.tree()?.clone();
            let to_query_reference = |reference: &Reference| {
                query_reference(reference)
                    .ok_or_else(|| unparsable(sql, item.expression.start, "expected a reference"))
            };
            Ok(match kind {
                ItemKind::Field(reference) => OutputColumn::Dimension(QueryDimension {
                    kind: QueryDimensionKind::Field {
                        reference: to_query_reference(reference)?,
                    },
                    output_name,
                    origin,
                }),
                ItemKind::Scalar => OutputColumn::Dimension(QueryDimension {
                    kind: QueryDimensionKind::Expression(tree),
                    output_name,
                    origin,
                }),
                ItemKind::Metric(reference) => OutputColumn::Metric(QueryMetric {
                    kind: QueryMetricKind::Reference(to_query_reference(reference)?),
                    output_name,
                    origin,
                }),
                ItemKind::MetricExpression => OutputColumn::Metric(QueryMetric {
                    kind: QueryMetricKind::Inline(tree),
                    output_name,
                    origin,
                }),
            })
        })
        .collect()
}

/// The reference `reference` as the parts of a query reference: its qualifier when written,
/// then its name.
fn query_reference(reference: &Reference) -> Option<QueryReference> {
    let first = reference.qualifier.as_ref().map(part);
    QueryReference::new(first.into_iter().chain([part(&reference.name)]).collect())
}

fn part(identifier: &Identifier) -> ReferencePart {
    if identifier.is_delimited() {
        ReferencePart::Delimited(identifier.text().to_owned())
    } else {
        ReferencePart::Regular(identifier.text().to_owned())
    }
}
