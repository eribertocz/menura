//! The instruction counts of `benchmarks/baseline.json` and the regression check of R7.

use std::collections::BTreeMap;

use crate::error::BenchmarkError;

/// Instruction counts by scenario name, in name order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstructionCounts(pub BTreeMap<String, u64>);

/// A scenario whose count fails R7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Regression {
    /// The count is more than 10 % above its baseline.
    AboveBaseline {
        /// The scenario.
        scenario: String,
        /// Its baseline count.
        baseline: u64,
        /// Its measured count.
        measured: u64,
    },
    /// The scenario has no baseline count.
    MissingBaseline(String),
}

impl InstructionCounts {
    /// Reads counts in the form [`InstructionCounts::render`] writes: one
    /// `"name": count` member per line inside `"instructions"`.
    ///
    /// # Errors
    ///
    /// [`BenchmarkError::Baseline`] for a member line that is not a quoted name and an
    /// unsigned integer.
    pub fn parse(text: &str) -> Result<Self, BenchmarkError> {
        let mut counts = BTreeMap::new();
        for (index, line) in text.lines().enumerate() {
            let member = line.trim().trim_end_matches(',');
            if member.starts_with('{') || member.starts_with('}') || member.is_empty() {
                continue;
            }
            let error = |message: &str| BenchmarkError::Baseline {
                line: index + 1,
                message: message.to_owned(),
            };
            let (name, value) = member
                .split_once(':')
                .ok_or_else(|| error("expected `\"name\": count`"))?;
            let name = name.trim().trim_matches('"');
            if name == "instructions" {
                continue;
            }
            let count = value
                .trim()
                .parse::<u64>()
                .map_err(|_| error("expected a count"))?;
            counts.insert(name.to_owned(), count);
        }
        Ok(Self(counts))
    }

    /// The counts as the JSON text of `benchmarks/baseline.json`.
    #[must_use]
    pub fn render(&self) -> String {
        let members: Vec<String> = self
            .0
            .iter()
            .map(|(name, count)| format!("    \"{name}\": {count}"))
            .collect();
        format!(
            "{{\n  \"instructions\": {{\n{}\n  }}\n}}\n",
            members.join(",\n")
        )
    }

    /// Every scenario of `self` (the measured counts) that fails R7 against `baseline`.
    ///
    /// # Example
    ///
    /// ```
    /// use menura_benchmarks::InstructionCounts;
    ///
    /// let baseline = InstructionCounts([("q".to_owned(), 100)].into());
    /// let measured = InstructionCounts([("q".to_owned(), 110)].into());
    /// assert!(measured.regressions(&baseline).is_empty());
    /// ```
    #[must_use]
    pub fn regressions(&self, baseline: &Self) -> Vec<Regression> {
        self.0
            .iter()
            .filter_map(|(scenario, measured)| match baseline.0.get(scenario) {
                None => Some(Regression::MissingBaseline(scenario.clone())),
                Some(base) if u128::from(*measured) * 10 > u128::from(*base) * 11 => {
                    Some(Regression::AboveBaseline {
                        scenario: scenario.clone(),
                        baseline: *base,
                        measured: *measured,
                    })
                }
                Some(_) => None,
            })
            .collect()
    }
}
