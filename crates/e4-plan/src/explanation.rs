//! The decisions E4 records in the explanation (PLN-7.3), in the order of that table: each
//! `Deduplicate` (E4-4.0e), each `left` join that can produce a NULL group (E4-4.1), each
//! `Join` along a temporal relationship (VAL-2.8, VAL-2.24), each context query (E4-8.2), then
//! each propagated row policy (E4-6.5, E4-6.10). The branches of
//! the context queries follow those of the query (E4-8.8).

use std::collections::BTreeSet;

use menura_contracts::types::common::Step;
use menura_model::explanation::{ExplanationEntry, Subject};
use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::paths::{Branch, Branches, QueryElement};
use menura_security::{AdmittedSecurityContext, policies_in_filter};

use crate::PlanInputs;
use crate::context::ContextQuery;
use crate::error::PlanError;
use crate::followed::PolicyGraph;
use crate::metrics::planned_metrics;
use crate::policy_visits::visit_order;
use crate::query_policies::visit_roots;

/// The entries of E4 for `plan`, the plan [`crate::build_plan`] built from `branches` and
/// `query` for the user of `security` over `model`.
///
/// # Errors
///
/// [`PlanError::Invariant`] for an input that [`crate::build_plan`] rejects the same way.
pub fn explanation<'input>(
    plan: &PlanNode,
    branches: &'input Branches,
    query: &'input BoundQuery,
    security: &AdmittedSecurityContext,
    model: &'input InternalModel,
) -> Result<Vec<ExplanationEntry>, PlanError> {
    let inputs = PlanInputs {
        branches,
        query,
        contexts: &[],
    };
    explanation_with_contexts(plan, &inputs, security, model)
}

/// The entries of E4 for `plan`, the plan [`crate::build_plan_with_contexts`] built from
/// `inputs` for the user of `security` over `model`.
///
/// # Errors
///
/// [`PlanError::Invariant`] for an input that [`crate::build_plan_with_contexts`] rejects the
/// same way.
pub fn explanation_with_contexts(
    plan: &PlanNode,
    inputs: &PlanInputs<'_>,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
) -> Result<Vec<ExplanationEntry>, PlanError> {
    let mut entries = deduplications(plan);
    let dimensions = inputs.query.columns().dimensions().count();
    for branch in inputs.branches.as_slice() {
        entries.extend(null_groups(branch, dimensions, model));
    }
    for context in inputs.contexts {
        let dimensions = context.context.query().columns().dimensions().count();
        for branch in context.branches.as_slice() {
            entries.extend(null_groups(branch, dimensions, model));
        }
    }
    let context_branches = inputs
        .contexts
        .iter()
        .flat_map(|context| context.branches.as_slice());
    for branch in inputs.branches.as_slice().iter().chain(context_branches) {
        entries.extend(temporal_joins(branch, model));
    }
    entries.extend(
        inputs
            .contexts
            .iter()
            .map(|context| context_query(context.context)),
    );
    entries.extend(propagated_policies(
        inputs.branches,
        inputs.query,
        security,
        model,
    )?);
    Ok(entries)
}

/// The `E4-8.2` entry of `context`, at the first context metric that needs it; its reason
/// names the modifier type and the base, never a value (PLN-7.3).
fn context_query(context: &ContextQuery) -> ExplanationEntry {
    ExplanationEntry {
        code: "E4-8.2",
        step: Step::E4,
        subject: Subject::Metric(context.metric().clone()),
        decision: "context query".to_owned(),
        reason: format!(
            "{} of {}: the base is evaluated in its own context query, attached to the result \
             rows by a Lookup",
            context.modifier_type(),
            context.base().as_str()
        ),
    }
}

/// One `E4-4.0e` entry per `Deduplicate` of `plan`, in branch order (E3-4.3).
fn deduplications(plan: &PlanNode) -> Vec<ExplanationEntry> {
    let mut entries = Vec::new();
    let mut pending = vec![plan];
    while let Some(node) = pending.pop() {
        if let Operation::Deduplicate { input, .. } = node.operation()
            && let Some(home) = home_scan(input)
        {
            entries.push(ExplanationEntry {
                code: "E4-4.0e",
                step: Step::E4,
                subject: Subject::Dataset(home.clone()),
                decision: "deduplication by (group, row identity)".to_owned(),
                reason: "the branch joins other datasets and computes metric components, so a \
                         home row counts once in each group it matches"
                    .to_owned(),
            });
        }
        pending.extend(node.inputs().into_iter().rev());
    }
    entries
}

/// The dataset of the home `Scan` under `node`: the first input of each node down to a
/// `Scan`.
fn home_scan(node: &PlanNode) -> Option<&NormalizedName> {
    let mut current = node;
    loop {
        if let Operation::Scan { dataset, .. } = current.operation() {
            return Some(dataset);
        }
        current = current.inputs().into_iter().next()?;
    }
}

/// One `E4-4.1` entry per join instance of `branch` on the path of one of its `dimensions`
/// dimensions, in path order: its `left` join keeps a home row without a visible match, whose
/// dimension values are NULL (E4-4.1).
fn null_groups(branch: &Branch, dimensions: usize, model: &InternalModel) -> Vec<ExplanationEntry> {
    let tree = &branch.join_tree;
    let mut on_dimension_paths: BTreeSet<usize> = BTreeSet::new();
    for position in 0..dimensions {
        let mut current = tree.bound_instance(&QueryElement::Dimension(position));
        while let Some(id) = current {
            on_dimension_paths.insert(id.index());
            current = tree.instance(id).and_then(|instance| instance.parent);
        }
    }
    let mut entries = Vec::new();
    for (index, instance) in tree.instances().iter().enumerate() {
        if !on_dimension_paths.contains(&index) {
            continue;
        }
        let temporal = model
            .relationship(&instance.step.relationship)
            .is_some_and(|relationship| relationship.temporal.is_some());
        let uncovered = if temporal {
            ", and so does a row whose time no version covers (VAL-2.10)"
        } else {
            ""
        };
        entries.push(ExplanationEntry {
            code: "E4-4.1",
            step: Step::E4,
            subject: Subject::Relationship(instance.step.relationship.clone()),
            decision: "the plan can produce a NULL group".to_owned(),
            reason: format!(
                "left join: a row of {} without a visible match in {} falls into the NULL group \
                 of the dimensions read through it{uncovered}",
                instance.step.from.as_str(),
                instance.step.to.as_str()
            ),
        });
    }
    entries
}

/// One `VAL-2.8` entry per join instance of `branch` along a temporal relationship, in path
/// order (VAL-2.24). Its reason names the `from_time` field and the population, never a value.
fn temporal_joins(branch: &Branch, model: &InternalModel) -> Vec<ExplanationEntry> {
    let mut entries = Vec::new();
    for instance in branch.join_tree.instances() {
        let Some(relationship) = model.relationship(&instance.step.relationship) else {
            continue;
        };
        let Some(range) = model.temporal_range(relationship) else {
            continue;
        };
        entries.push(ExplanationEntry {
            code: "VAL-2.8",
            step: Step::E4,
            subject: Subject::Relationship(relationship.name.clone()),
            decision: "temporal join".to_owned(),
            reason: format!(
                "a row of {} matches the versions of {} in population {} that cover its field \
                 {}",
                relationship.from.as_str(),
                relationship.to.as_str(),
                range.population.as_str(),
                range.from_time.as_str()
            ),
        });
    }
    entries
}

/// The part of the reason of an `E4-6.10` entry that names the temporal relationships the
/// visible rows of `roots` follow to `name` (VAL-2.17); empty when they follow none.
fn temporal_followed(
    roots: &[&NormalizedName],
    name: &NormalizedName,
    graph: &PolicyGraph<'_>,
    model: &InternalModel,
) -> String {
    let temporal: BTreeSet<&NormalizedName> = roots
        .iter()
        .flat_map(|root| graph.temporal_towards(root, name, model))
        .collect();
    if temporal.is_empty() {
        return String::new();
    }
    let names: Vec<&str> = temporal.iter().map(|name| name.as_str()).collect();
    format!(
        "; follows the temporal relationships {}, through the versions that cover each row's \
         time",
        names.join(", ")
    )
}

/// One `E4-6.10` entry per row policy that the visible rows of a home dataset, or of the
/// population dataset of a semi-additive branch, take from another dataset (E4-6.5), in the
/// visit order of E4-6.9. Its reason names every such dataset it reaches.
fn propagated_policies<'input>(
    branches: &'input Branches,
    query: &'input BoundQuery,
    security: &AdmittedSecurityContext,
    model: &'input InternalModel,
) -> Result<Vec<ExplanationEntry>, PlanError> {
    let metrics = planned_metrics(query, model)?;
    let roots = visit_roots(branches, &metrics, model)?;
    let graph = PolicyGraph::new(model);
    let mut reached: Vec<(&NormalizedName, Vec<&NormalizedName>)> = Vec::new();
    for root in roots {
        for dataset in visit_order(root, model, &graph).into_iter().skip(1) {
            match reached.iter_mut().find(|(name, _)| *name == &dataset.name) {
                Some((_, roots)) if !roots.contains(&&root.name) => roots.push(&root.name),
                Some(_) => {}
                None => reached.push((&dataset.name, vec![&root.name])),
            }
        }
    }
    let mut entries = Vec::new();
    for (name, roots) in reached {
        let Some(dataset) = model.dataset(name) else {
            continue;
        };
        let reached_names: Vec<&str> = roots.iter().map(|root| root.as_str()).collect();
        let followed = temporal_followed(&roots, name, &graph, model);
        for policy in policies_in_filter(dataset, security) {
            entries.push(ExplanationEntry {
                code: "E4-6.10",
                step: Step::E4,
                subject: Subject::Policy {
                    dataset: name.clone(),
                    policy: policy.name.clone(),
                },
                decision: "row policy propagated".to_owned(),
                reason: format!(
                    "a row is visible only if its related rows of {} are visible; reaches {}\
                     {followed}",
                    name.as_str(),
                    reached_names.join(", ")
                ),
            });
        }
    }
    Ok(entries)
}
