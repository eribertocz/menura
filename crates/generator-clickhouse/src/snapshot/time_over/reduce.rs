//! One value per (result group, entity) from the values of a time `Snapshot` (ADD-4.1, E4-7.1,
//! E4-7.2), with one more row that raises the first runtime condition met (E6-4.6b).

use menura_model::internal_model::{
    NullHandling, Selection, SnapshotAggregation, StaleHandling, UnexpectedHandling,
};

use super::Steps;
use menura_representations::logical_plan::SnapshotMissing;

use super::frame::{EXPECTED, Frame, PERIOD, STALE, VIOLATION, qualified};
use super::values::{VALUE, expected_join};
use crate::additivity_error::AdditivityCondition;
use crate::body::{SelectBody, union_all};
use crate::error::GenerateError;
use crate::null_safe::join_equal;
use crate::snapshot::group_by;
use crate::steps::{StepWriter, column_type};

/// The alias of a candidate value.
const CANDIDATE: &str = "\"menura_candidate\"";
/// The alias of the period `global_period` selects.
const SELECTED: &str = "\"menura_selected\"";

impl StepWriter<'_> {
    /// The body of the node: the reduction of the candidate values, then, when a runtime
    /// condition can occur, one row that raises the first one met.
    pub(super) fn reduce(
        &mut self,
        frame: &Frame<'_>,
        steps: &Steps,
    ) -> Result<SelectBody, GenerateError> {
        let candidates = match &steps.filled {
            Some(filled) => {
                let columns = frame.value_columns();
                let union = union_all(
                    select(&columns, &steps.checked),
                    &[select(&columns, filled)],
                );
                self.push_step("candidates", union)
            }
            None => steps.checked.clone(),
        };
        let body = match frame.rule.aggregation {
            SnapshotAggregation::Minimum => combined(frame, &candidates, "MIN")?,
            SnapshotAggregation::Maximum => combined(frame, &candidates, "MAX")?,
            SnapshotAggregation::Average => combined(frame, &candidates, "AVG")?,
            SnapshotAggregation::Last(Selection::GlobalPeriod) => {
                self.global_period(frame, &candidates, &steps.values, "MAX")?
            }
            SnapshotAggregation::First(Selection::GlobalPeriod) => {
                self.global_period(frame, &candidates, &steps.values, "MIN")?
            }
            SnapshotAggregation::Last(Selection::PerEntity) => {
                per_entity(frame, &candidates, "DESC")?
            }
            SnapshotAggregation::First(Selection::PerEntity) => {
                per_entity(frame, &candidates, "ASC")?
            }
        };
        let branches = violations(frame, steps);
        if branches.is_empty() {
            return Ok(body);
        }
        let (first, others) = branches.split_at(1);
        let violations = union_all(first[0].clone(), others);
        let violations = self.push_step("violations", violations);
        Ok(union_all(body, &[raise(frame, &violations)]))
    }

    /// The value of each entity in the last (`MAX`) or first (`MIN`) period with data of its
    /// result group: the periods of the rows of the input, NULL values included (E4-7.3,
    /// ADD-4.8).
    fn global_period(
        &mut self,
        frame: &Frame<'_>,
        candidates: &str,
        values: &str,
        function: &str,
    ) -> Result<SelectBody, GenerateError> {
        let mut items = frame.keys.clone();
        let grouped = items.len();
        items.push(format!("{function}({PERIOD}) AS {PERIOD}"));
        let selected = SelectBody {
            items,
            from: values.to_owned(),
            clauses: group_by(grouped),
        };
        let selected = self.push_step("selected", selected);
        let mut condition: Vec<String> = frame
            .keys
            .iter()
            .map(|key| join_equal(&format!("{CANDIDATE}.{key}"), &format!("{SELECTED}.{key}")))
            .collect();
        condition.push(format!("{CANDIDATE}.{PERIOD} = {SELECTED}.{PERIOD}"));
        let mut items = qualified(CANDIDATE, &frame.keys);
        items.extend(qualified(CANDIDATE, &frame.entity));
        items.extend(cast_components(frame, |name| {
            format!("{CANDIDATE}.{name}")
        })?);
        Ok(SelectBody {
            items,
            from: format!("{candidates} AS {CANDIDATE}"),
            clauses: vec![format!(
                "JOIN {selected} AS {SELECTED} ON {}",
                condition.join(" AND ")
            )],
        })
    }
}

/// `SELECT columns FROM from`.
fn select(columns: &[String], from: &str) -> SelectBody {
    SelectBody {
        items: columns.to_vec(),
        from: from.to_owned(),
        clauses: Vec::new(),
    }
}

/// Each component, cast to its column type in the node (E6-4.5), its value written by `value`.
fn cast_components(
    frame: &Frame<'_>,
    value: impl Fn(&str) -> String,
) -> Result<Vec<String>, GenerateError> {
    frame
        .rule
        .components
        .iter()
        .zip(&frame.components)
        .map(|(component, name)| {
            let result_type = column_type(frame.node, component.name.as_str())?;
            Ok(format!("CAST({} AS {result_type}) AS {name}", value(name)))
        })
        .collect()
}

/// `function` of each entity's values across its periods, per (keys, entity) (E4-7.2).
fn combined(
    frame: &Frame<'_>,
    candidates: &str,
    function: &str,
) -> Result<SelectBody, GenerateError> {
    let mut items: Vec<String> = frame.keys.iter().chain(&frame.entity).cloned().collect();
    let grouped = items.len();
    items.extend(cast_components(frame, |name| {
        format!("{function}({name})")
    })?);
    Ok(SelectBody {
        items,
        from: candidates.to_owned(),
        clauses: group_by(grouped),
    })
}

/// Each entity's value in its own last (`DESC`) or first (`ASC`) period with a value.
fn per_entity(
    frame: &Frame<'_>,
    candidates: &str,
    direction: &str,
) -> Result<SelectBody, GenerateError> {
    let mut items: Vec<String> = frame.keys.iter().chain(&frame.entity).cloned().collect();
    let partition = items.join(", ");
    items.extend(cast_components(frame, str::to_owned)?);
    Ok(SelectBody {
        items,
        from: candidates.to_owned(),
        clauses: vec![format!(
            "QUALIFY ROW_NUMBER() OVER (PARTITION BY {partition} ORDER BY {PERIOD} {direction} NULLS LAST) = 1"
        )],
    })
}

/// One branch per runtime condition the rule turns into an error, in code order (ADD-7): each
/// row is the text of a condition met.
fn violations(frame: &Frame<'_>, steps: &Steps) -> Vec<SelectBody> {
    let metric = &frame.rule.metric;
    let text = |condition: AdditivityCondition| {
        format!("{} AS {VIOLATION}", condition.message_sql(metric))
    };
    let mut branches = Vec::new();
    if let (SnapshotMissing::Error, Some(missing)) = (&frame.time.missing, &steps.missing) {
        branches.push(select(&[text(AdditivityCondition::MissingEntity)], missing));
    }
    if frame.time.unexpected == UnexpectedHandling::Error {
        branches.push(SelectBody {
            items: vec![text(AdditivityCondition::UnexpectedRow)],
            from: format!("{} AS {VALUE}", steps.values),
            clauses: vec![
                expected_join(frame),
                format!("WHERE {EXPECTED}.{} IS NULL", frame.expected_period),
            ],
        });
    }
    if frame.rule.nulls == NullHandling::Error {
        let any_null: Vec<String> = frame
            .components
            .iter()
            .map(|component| format!("{component} IS NULL"))
            .collect();
        let mut branch = select(&[text(AdditivityCondition::NullValue)], &steps.checked);
        branch
            .clauses
            .push(format!("WHERE {}", any_null.join(" OR ")));
        branches.push(branch);
    }
    let stale_error = matches!(
        frame.time.missing,
        SnapshotMissing::CarryForward {
            on_stale: StaleHandling::Error,
            ..
        }
    );
    if let (true, Some(filled)) = (stale_error, &steps.filled) {
        let mut branch = select(&[text(AdditivityCondition::StaleValue)], filled);
        branch.clauses.push(format!("WHERE {STALE}"));
        branches.push(branch);
    }
    branches
}

/// One row, when a condition is met, whose components raise the first one in code order: the
/// texts sort in code order (E6-4.6b), and `throwIf` takes a constant message, so each
/// condition is raised when it is the first (E6-7.5). The components are then NULL.
fn raise(frame: &Frame<'_>, violations: &str) -> SelectBody {
    let mut items: Vec<String> = frame
        .keys
        .iter()
        .chain(&frame.entity)
        .map(|name| format!("NULL AS {name}"))
        .collect();
    let metric = &frame.rule.metric;
    let raised: Vec<String> = AdditivityCondition::ALL
        .iter()
        .map(|condition| {
            let met = format!("min({VIOLATION}) = {}", condition.message_sql(metric));
            condition.raise_sql(metric, &met)
        })
        .collect();
    let value = format!("nullIf({}, 0)", raised.join(" + "));
    for name in &frame.components {
        items.push(format!("{value} AS {name}"));
    }
    SelectBody {
        items,
        from: violations.to_owned(),
        clauses: vec![format!("HAVING min({VIOLATION}) IS NOT NULL")],
    }
}
