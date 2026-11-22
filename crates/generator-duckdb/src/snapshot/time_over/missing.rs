//! The expected (entity, period) pairs without a value (ADD-4.2, E4-7.7): every such pair, or
//! under `selection: per_entity` the pairs of the edge period of a unit whose entity has no
//! value in the unit.

use menura_representations::logical_plan::SnapshotMissing;

use super::frame::{EXPECTED, Frame, PERIOD, UNIT, equalities, qualified, truncated};
use super::per_entity_edge;
use crate::body::SelectBody;
use crate::day::grain_name;
use crate::snapshot::group_by;
use crate::steps::StepWriter;

/// The alias of a present (entity, period) or (entity, unit).
const PRESENT: &str = "\"menura_present\"";
/// The alias of the edge period of a unit.
const EDGE: &str = "\"menura_edge\"";

impl StepWriter<'_> {
    /// The step of the missing pairs, named by their entity columns and `menura.period`;
    /// `None` under `missing: exclude`, which never reads them.
    pub(super) fn missing(&mut self, frame: &Frame<'_>, checked: &str) -> Option<String> {
        if frame.time.missing == SnapshotMissing::Exclude {
            return None;
        }
        let unit = frame.time.unit.as_ref().map(|key| grain_name(key.grain));
        let edge = per_entity_edge(frame.rule.aggregation);
        let mut present = frame.entity.clone();
        match (edge, unit) {
            (None, _) => present.push(PERIOD.to_owned()),
            (Some(_), Some(grain)) => {
                present.push(format!("{} AS {UNIT}", truncated(PERIOD, grain)));
            }
            (Some(_), None) => {}
        }
        let present = SelectBody {
            clauses: group_by(present.len()),
            items: present,
            from: checked.to_owned(),
        };
        let present = self.push_step("present", present);
        let body = match edge {
            None => all_missing(frame, &present),
            Some(function) => {
                let edges = self.push_step("edges", edges(frame, function, unit));
                edge_missing(frame, &present, &edges, unit.is_some())
            }
        };
        Some(self.push_step("missing", body))
    }
}

/// The expected pairs and their alias, with the missing pairs' columns.
fn expected_items(frame: &Frame<'_>) -> Vec<String> {
    let mut items: Vec<String> = qualified(EXPECTED, &frame.expected_entity)
        .into_iter()
        .zip(&frame.entity)
        .map(|(column, name)| format!("{column} AS {name}"))
        .collect();
    items.push(format!("{} AS {PERIOD}", frame.expected_day));
    items
}

/// Every expected pair without a present (entity, period).
fn all_missing(frame: &Frame<'_>, present: &str) -> SelectBody {
    let entity = equalities(PRESENT, &frame.entity, EXPECTED, &frame.expected_entity);
    SelectBody {
        items: expected_items(frame),
        from: format!("{} AS {EXPECTED}", frame.expected),
        clauses: vec![
            format!(
                "LEFT JOIN {present} AS {PRESENT} ON {entity} AND {PRESENT}.{PERIOD} = {}",
                frame.expected_day
            ),
            format!("WHERE {}", absent(frame)),
        ],
    }
}

/// The last (`MAX`) or first (`MIN`) period of each unit of the query's periods.
fn edges(frame: &Frame<'_>, function: &str, unit: Option<&str>) -> SelectBody {
    let (periods, column) = &frame.periods;
    let mut items = Vec::new();
    if let Some(grain) = unit {
        items.push(format!("{} AS {UNIT}", truncated(column, grain)));
    }
    let grouped = items.len();
    items.push(format!("{function}({column}) AS {PERIOD}"));
    SelectBody {
        items,
        from: periods.clone(),
        clauses: group_by(grouped),
    }
}

/// The expected pairs of the edge period of each unit whose entity has no value in the unit.
fn edge_missing(frame: &Frame<'_>, present: &str, edges: &str, has_unit: bool) -> SelectBody {
    let entity = equalities(PRESENT, &frame.entity, EXPECTED, &frame.expected_entity);
    let condition = if has_unit {
        format!("{entity} AND {PRESENT}.{UNIT} = {EDGE}.{UNIT}")
    } else {
        entity
    };
    SelectBody {
        items: expected_items(frame),
        from: format!("{} AS {EXPECTED}", frame.expected),
        clauses: vec![
            format!(
                "JOIN {edges} AS {EDGE} ON {} = {EDGE}.{PERIOD}",
                frame.expected_day
            ),
            format!("LEFT JOIN {present} AS {PRESENT} ON {condition}"),
            format!("WHERE {}", absent(frame)),
        ],
    }
}

/// `present.e1 IS NULL`: no present row matched, since a matched entity is never NULL.
fn absent(frame: &Frame<'_>) -> String {
    let first = frame.entity.first().map_or("", String::as_str);
    format!("{PRESENT}.{first} IS NULL")
}
