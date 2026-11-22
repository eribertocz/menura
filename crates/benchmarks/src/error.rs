//! The failures of the benchmarks, each with the rule ID of section 5.6 it concerns.

use std::fmt;

/// A failure that stops a benchmark run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkError {
    /// A generated model does not load: its diagnostic codes (R5).
    ModelRejected {
        /// The name of the model.
        model: &'static str,
        /// The codes of the errors `load` returned.
        codes: Vec<String>,
    },
    /// A generated query does not compile to a plan: its diagnostic codes (R5).
    QueryRejected {
        /// The name of the query.
        query: String,
        /// The codes of the errors `compile` returned.
        codes: Vec<String>,
    },
    /// No scenario has this name (R7).
    UnknownScenario(String),
    /// `benchmarks/baseline.json` is not in the form `write-baseline` writes (R7).
    Baseline {
        /// The line, counted from 1.
        line: usize,
        /// What is wrong with it.
        message: String,
    },
    /// A file could not be read or written, or Valgrind could not be run (R7).
    Io(String),
    /// The command line is not one of the commands of the benchmark binary (R4).
    Usage(String),
}

impl BenchmarkError {
    /// The rule ID of section 5.6 the failure concerns.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::ModelRejected { .. } | Self::QueryRejected { .. } => "R5",
            Self::UnknownScenario(_) | Self::Baseline { .. } | Self::Io(_) => "R7",
            Self::Usage(_) => "R4",
        }
    }
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: ", self.code())?;
        match self {
            Self::ModelRejected { model, codes } => {
                write!(
                    formatter,
                    "model `{model}` does not load: {}",
                    codes.join(", ")
                )
            }
            Self::QueryRejected { query, codes } => {
                write!(
                    formatter,
                    "query `{query}` does not compile: {}",
                    codes.join(", ")
                )
            }
            Self::UnknownScenario(name) => write!(formatter, "no scenario is named `{name}`"),
            Self::Baseline { line, message } => {
                write!(formatter, "baseline line {line}: {message}")
            }
            Self::Io(message) | Self::Usage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for BenchmarkError {}
