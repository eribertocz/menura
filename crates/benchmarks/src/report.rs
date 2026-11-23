//! Runs the benchmarks and reports each measure with the rule it checks (R1–R3, R6, R7).

use std::path::Path;
use std::time::Duration;

use crate::baseline::{InstructionCounts, Regression};
use crate::callgrind::count_compilation;
use crate::error::BenchmarkError;
use crate::inputs::ModelInputs;
use crate::model_shape::ModelShape;
use crate::queries::{SWEEP_ITEMS, reference_queries, security_context_document, sweep_query};
use crate::scenarios::scenario_queries;
use crate::targets::{r1_passes, r2_passes, r3_violations};
use crate::wall_clock::{time_compile, time_load};

/// The lines of a benchmark report and the failed checks among them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// One Markdown table row per measure.
    pub lines: Vec<String>,
    /// One line per failed check, starting with its rule ID.
    pub failures: Vec<String>,
}

impl Report {
    fn measure(&mut self, rule: &str, name: &str, value: &str, passed: bool) {
        let verdict = if passed { "pass" } else { "FAIL" };
        self.lines
            .push(format!("| {rule} | {name} | {value} | {verdict} |"));
        if !passed {
            self.failures.push(format!("{rule}: {name} ({value})"));
        }
    }

    /// The report as a Markdown table.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::from("| Rule | Measure | Value | Verdict |\n|---|---|---|---|\n");
        for line in &self.lines {
            text.push_str(line);
            text.push('\n');
        }
        text
    }
}

fn milliseconds(time: Duration) -> String {
    format!("{:.3} ms", time.as_secs_f64() * 1000.0)
}

/// The wall-clock measures of R6: `load` of the R1 model (R1) and the 99th percentile of
/// each reference query (R2).
///
/// # Errors
///
/// A model that does not load or a query that does not compile to a plan.
pub fn wall_clock_report() -> Result<Report, BenchmarkError> {
    let mut report = Report::default();
    let r1_time = time_load(&ModelInputs::of(&ModelShape::r1()))?;
    report.measure(
        "R1",
        "load of the R1 model",
        &milliseconds(r1_time),
        r1_passes(r1_time),
    );
    let shape = ModelShape::reference();
    let handle = ModelInputs::of(&shape).load()?;
    let context = security_context_document();
    for query in reference_queries(&shape) {
        let time = time_compile(&handle, &query, &context)?;
        report.measure("R2", &query.name, &milliseconds(time), r2_passes(time));
    }
    Ok(report)
}

/// The instruction counts of every scenario (R7), the R3 sweep check on them, and, with a
/// `baseline`, the regression check of R7. Returns the report and the measured counts.
///
/// # Errors
///
/// The errors of [`count_compilation`].
pub fn instruction_report(
    executable: &Path,
    directory: &Path,
    baseline: Option<&InstructionCounts>,
) -> Result<(Report, InstructionCounts), BenchmarkError> {
    let mut counts = InstructionCounts::default();
    for query in scenario_queries() {
        let count = count_compilation(executable, &query.name, directory)?;
        counts.0.insert(query.name, count);
    }
    let mut report = Report::default();
    sweep_check(&counts, &mut report);
    if let Some(baseline) = baseline {
        regression_check(&counts, baseline, &mut report);
    }
    Ok((report, counts))
}

fn sweep_check(counts: &InstructionCounts, report: &mut Report) {
    let shape = ModelShape::reference();
    let mut points = Vec::with_capacity(SWEEP_ITEMS.len());
    for items in SWEEP_ITEMS {
        let name = format!("sweep with {items} dimensions and {items} metrics");
        match counts.0.get(&sweep_query(&shape, items).name) {
            Some(count) => points.push((items, *count)),
            None => report.measure("R3", &name, "not measured", false),
        }
    }
    if points.len() < SWEEP_ITEMS.len() {
        return;
    }
    let violations = r3_violations(&points);
    for (items, count) in points {
        let name = format!("sweep with {items} dimensions and {items} metrics");
        report.measure(
            "R3",
            &name,
            &format!("{count} Ir"),
            !violations.contains(&items),
        );
    }
}

fn regression_check(counts: &InstructionCounts, baseline: &InstructionCounts, report: &mut Report) {
    let regressions = counts.regressions(baseline);
    for (scenario, count) in &counts.0 {
        let base = baseline.0.get(scenario).copied().unwrap_or_default();
        let failed = regressions.iter().any(|regression| match regression {
            Regression::AboveBaseline { scenario: name, .. }
            | Regression::MissingBaseline(name)
            | Regression::StaleBaseline(name) => name == scenario,
        });
        let value = format!("{count} Ir (baseline {base})");
        report.measure("R7", scenario, &value, !failed);
    }
    for regression in &regressions {
        if let Regression::StaleBaseline(scenario) = regression {
            report.measure("R7", scenario, "baseline entry of no scenario", false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InstructionCounts, Report, regression_check, sweep_check};

    #[test]
    fn r3_missing_sweep_count_fails() {
        let mut report = Report::default();
        sweep_check(&InstructionCounts::default(), &mut report);
        assert_eq!(report.failures.len(), 7);
        assert!(report.failures[0].contains("not measured"));
    }

    #[test]
    fn r7_stale_baseline_entry_fails() {
        let baseline = InstructionCounts([("gone".to_owned(), 1)].into());
        let mut report = Report::default();
        regression_check(&InstructionCounts::default(), &baseline, &mut report);
        assert_eq!(
            report.failures,
            vec!["R7: gone (baseline entry of no scenario)".to_owned()]
        );
    }
}
