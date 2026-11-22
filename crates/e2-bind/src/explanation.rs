//! The decisions E2 records in the explanation (PLN-7.3): the roles and attributes of the
//! security context that the model does not declare (SEC-3.2, SEC-3.2a), and a resolved
//! relative date range (QRY-4.2).

use menura_contracts::types::common::{Grain, Step};
use menura_contracts::types::query::{DateBounds, RelativeRange};
use menura_model::explanation::{ExplanationEntry, Subject};
use menura_representations::bound_query::BoundQuery;
use menura_representations::query_tree::QueryTree;
use menura_security::AdmittedSecurityContext;

/// The entries of E2 for the query `tree`, bound as `bound`, for the user of `context`, in
/// the order of PLN-7.3: each undeclared role, then each undeclared attribute, both in
/// code-point order (SEC-3.2, SEC-3.2a), then the resolved relative range, if any (QRY-4.2).
/// No entry holds a value: neither attribute values nor the dates of the range (PLN-4.2).
#[must_use]
pub fn explanation(
    tree: &QueryTree,
    bound: &BoundQuery,
    context: &AdmittedSecurityContext,
) -> Vec<ExplanationEntry> {
    let roles = context.ignored_roles().iter().map(|role| ExplanationEntry {
        code: "SEC-3.2",
        step: Step::E2,
        subject: Subject::Role(role.as_str().to_owned()),
        decision: "role ignored".to_owned(),
        reason: "the model does not declare the role, so it grants nothing".to_owned(),
    });
    let attributes = context
        .ignored_attributes()
        .iter()
        .map(|name| ExplanationEntry {
            code: "SEC-3.2a",
            step: Step::E2,
            subject: Subject::Attribute(name.clone()),
            decision: "attribute ignored".to_owned(),
            reason: "the model does not declare the attribute, so no policy reads it".to_owned(),
        });
    roles
        .chain(attributes)
        .chain(resolved_range(tree, bound))
        .collect()
}

/// The `QRY-4.2` entry of a relative `date_range` of `tree` that E2 resolved in `bound`.
fn resolved_range(tree: &QueryTree, bound: &BoundQuery) -> Option<ExplanationEntry> {
    let DateBounds::Relative(relative) = &tree.date_range()?.bounds else {
        return None;
    };
    let field = bound.date_range()?.field.clone();
    Some(ExplanationEntry {
        code: "QRY-4.2",
        step: Step::E2,
        subject: Subject::Field(field),
        decision: "relative range resolved to an absolute range".to_owned(),
        reason: range_reason(relative),
    })
}

/// Why the range has its bounds: its unit, count and `include_current`, never its dates.
fn range_reason(relative: &RelativeRange) -> String {
    let current = if relative.include_current.unwrap_or(true) {
        "including"
    } else {
        "excluding"
    };
    format!(
        "{} {} {current} the current one, from reference_time in time_zone; the bounds are \
         bound parameters",
        relative.count,
        unit_name(relative.unit, relative.count),
    )
}

/// The English name of `unit`, plural unless `count` is 1.
fn unit_name(unit: Grain, count: u64) -> &'static str {
    let (one, many) = match unit {
        Grain::Day => ("day", "days"),
        Grain::Week => ("week", "weeks"),
        Grain::Month => ("month", "months"),
        Grain::Quarter => ("quarter", "quarters"),
        Grain::Year => ("year", "years"),
    };
    if count == 1 { one } else { many }
}
