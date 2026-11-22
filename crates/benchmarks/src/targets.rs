//! The pass conditions of R1, R2 and R3 (R6).

use std::time::Duration;

/// The time within which `load` of the R1 model must succeed (R1, R6).
pub const R1_LOAD_LIMIT: Duration = Duration::from_secs(2);
/// The 99th-percentile compile time every reference query must stay under (R2, R6).
pub const R2_COMPILE_LIMIT: Duration = Duration::from_millis(10);

/// Whether `load` of the R1 model in `time` meets R1.
#[must_use]
pub fn r1_passes(time: Duration) -> bool {
    time < R1_LOAD_LIMIT
}

/// Whether a reference query whose 99th percentile is `time` meets R2.
#[must_use]
pub fn r2_passes(time: Duration) -> bool {
    time < R2_COMPILE_LIMIT
}

/// The item counts of the R3 sweep at which a doubling more than doubled the cost plus 10 %:
/// for each consecutive pair of `(items, cost)` points, the larger item count when
/// `next > 2 × previous × 1.1`.
///
/// # Example
///
/// ```
/// use menura_benchmarks::r3_violations;
///
/// assert!(r3_violations(&[(1, 100), (2, 220), (4, 440)]).is_empty());
/// assert_eq!(r3_violations(&[(1, 100), (2, 221)]), vec![2]);
/// ```
#[must_use]
pub fn r3_violations(points: &[(usize, u64)]) -> Vec<usize> {
    points
        .windows(2)
        .filter(|pair| u128::from(pair[1].1) * 10 > u128::from(pair[0].1) * 22)
        .map(|pair| pair[1].0)
        .collect()
}
