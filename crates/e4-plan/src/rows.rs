//! The rows of a branch before its aggregation (E4-4 steps 1 to 5): the home `Scan` of its
//! visible rows, the joins of its join tree, the query-filter `Filter` and, when the branch
//! joins and has components, the `Deduplicate`.

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, InternalModel, NormalizedName};
use menura_representations::logical_plan::{AggregateComponent, NamedExpression, PlanNode};
use menura_representations::origin::{Origin, QueryLocation};
use menura_representations::paths::Branch;
use menura_security::AdmittedSecurityContext;

use crate::columns::read_fields;
use crate::error::PlanError;
use crate::layout::Layout;
use crate::nodes::{component_expression, deduplicate, filter};
use crate::{followed, joins, policy_visits, scan, visibility};

/// Where the rows of a branch are read from.
pub(crate) struct BranchSource<'input> {
    /// The branch.
    pub(crate) branch: &'input Branch,
    /// Its home dataset.
    pub(crate) home: &'input Dataset,
    /// Its join instances and their columns.
    pub(crate) layout: &'input Layout<'input>,
    /// The user whose row policies apply.
    pub(crate) security: &'input AdmittedSecurityContext,
    /// The model.
    pub(crate) model: &'input InternalModel,
    /// The admitted catalog schema.
    pub(crate) catalog: &'input CatalogSchemaDocument,
    /// Whether the query has a `reference_time` (E4-6.9b).
    pub(crate) has_reference_time: bool,
}

/// What the rows must provide, and which rows to keep.
pub(crate) struct RowsRequest<'request> {
    /// The `Aggregate` keys of the branch.
    pub(crate) keys: &'request [NamedExpression],
    /// The components of the branch.
    pub(crate) components: &'request [AggregateComponent],
    /// Other home fields the rows must provide, by normalized name.
    pub(crate) fields: &'request [NormalizedName],
    /// The query-filter predicate, with the element it comes from.
    pub(crate) predicate: Option<(Expression, QueryLocation)>,
}

/// The rows of the branch of `source` for `request`: its visible home rows, joined along its
/// join tree, filtered by the predicate, then deduplicated per (key values, home row) when the
/// branch joins and has components (E4-4.0a to E4-4.0f).
///
/// # Errors
///
/// `ESEC-02` and `EQRY-03` from the row policies (E4-6.9, E4-6.9b), and an internal error for
/// an input the stages before E4 never produce.
pub(crate) fn branch_rows(
    source: &BranchSource<'_>,
    request: RowsRequest<'_>,
) -> Result<PlanNode, PlanError> {
    let BranchSource {
        branch,
        home,
        layout,
        security,
        model,
        catalog,
        has_reference_time,
    } = *source;
    let graph = followed::PolicyGraph::new(model);
    let row_filters =
        policy_visits::row_filters(home, model, &graph, security, has_reference_time)?;
    let mut read = fields_read(
        request.keys,
        request.components,
        request.predicate.as_ref().map(|(predicate, _)| predicate),
    );
    read.extend(request.fields.iter().cloned());
    let visibility = visibility::Visibility {
        model,
        graph: &graph,
        filters: &row_filters,
        catalog,
    };
    read.extend(joins::join_reads(layout));
    read.extend(visibility.fields(home)?);
    for instance in layout.instances() {
        read.extend(visibility.instance_fields(instance.dataset, &instance.prefix)?);
    }
    let deduplicated = !branch.join_tree.is_empty() && !request.components.is_empty();
    let identity = deduplicated.then(|| layout.identity());
    let scan = scan::scan(home, &joins::home_reads(layout, &read), catalog, identity)?;
    let mut node = visibility.visible_rows(scan, home, None)?;
    node = joins::join_instances(node, layout, &read, &visibility)?;
    if let Some((predicate, location)) = request.predicate {
        node = filter(node, predicate, Origin::Query(location));
    }
    if let Some(identity) = identity {
        let origin = Origin::Model(home.location.clone());
        node = deduplicate(node, request.keys, identity.clone(), origin);
    }
    Ok(node)
}

/// The fields the keys, the components and the query predicate read.
fn fields_read(
    keys: &[NamedExpression],
    components: &[AggregateComponent],
    query_predicate: Option<&Expression>,
) -> Vec<NormalizedName> {
    let mut read = Vec::new();
    for key in keys {
        read_fields(&key.expression, &mut read);
    }
    for component in components {
        read_fields(&component_expression(component), &mut read);
    }
    if let Some(predicate) = query_predicate {
        read_fields(predicate, &mut read);
    }
    read
}
