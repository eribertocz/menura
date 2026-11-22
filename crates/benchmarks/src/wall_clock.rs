//! Wall-clock measures of R6.

use std::time::{Duration, Instant};

use menura_core::Handle;

use crate::error::BenchmarkError;
use crate::inputs::{ModelInputs, compile_query};
use crate::queries::BenchmarkQuery;

/// Compilations of a query before it is measured (R6).
pub const WARM_UP_COMPILATIONS: usize = 20;
/// Measured compilations of a query (R6).
pub const MEASURED_COMPILATIONS: usize = 200;

/// The 99th percentile of `times` by the nearest-rank method; zero for no times.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use menura_benchmarks::percentile_99;
///
/// let times: Vec<Duration> = (1..=200).map(Duration::from_millis).collect();
/// assert_eq!(percentile_99(&times), Duration::from_millis(198));
/// ```
#[must_use]
pub fn percentile_99(times: &[Duration]) -> Duration {
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len() * 99).div_ceil(100);
    sorted
        .get(rank.saturating_sub(1))
        .copied()
        .unwrap_or_default()
}

/// The wall-clock time of one `load` of `inputs` (R1).
///
/// # Errors
///
/// [`BenchmarkError::ModelRejected`] when the model does not load.
pub fn time_load(inputs: &ModelInputs) -> Result<Duration, BenchmarkError> {
    let start = Instant::now();
    inputs.load()?;
    Ok(start.elapsed())
}

/// The 99th percentile of the wall-clock times of `query`: compiled
/// [`WARM_UP_COMPILATIONS`] times, then [`MEASURED_COMPILATIONS`] measured times (R6).
///
/// # Errors
///
/// [`BenchmarkError::QueryRejected`] when the query does not compile to a plan.
pub fn time_compile(
    handle: &Handle,
    query: &BenchmarkQuery,
    context: &[u8],
) -> Result<Duration, BenchmarkError> {
    for _ in 0..WARM_UP_COMPILATIONS {
        compile_query(handle, query, context)?;
    }
    let mut times = Vec::with_capacity(MEASURED_COMPILATIONS);
    for _ in 0..MEASURED_COMPILATIONS {
        let start = Instant::now();
        compile_query(handle, query, context)?;
        times.push(start.elapsed());
    }
    Ok(percentile_99(&times))
}
