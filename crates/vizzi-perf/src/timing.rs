//! Nearest-rank summaries for small, explicitly reported measurement samples.

use serde::Serialize;
use std::time::Duration;

/// Wall-clock sample summary; microseconds are used throughout.
#[derive(Debug, PartialEq, Serialize)]
pub struct Summary {
    /// Number of measurements.
    pub samples: usize,
    /// Smallest observed duration.
    pub min_us: f64,
    /// Nearest-rank median.
    pub p50_us: f64,
    /// Nearest-rank 95th percentile.
    pub p95_us: f64,
    /// Largest observed duration.
    pub max_us: f64,
}

/// Summarize nonempty samples, retaining zero-duration observations.
pub fn summarize(samples: &[Duration]) -> Option<Summary> {
    if samples.is_empty() {
        return None;
    }
    let mut ordered = samples.to_vec();
    ordered.sort();
    let microseconds = |index: usize| ordered[index].as_nanos() as f64 / 1000.0;
    Some(Summary {
        samples: ordered.len(),
        min_us: microseconds(0),
        p50_us: microseconds((ordered.len() - 1) / 2),
        p95_us: microseconds((ordered.len() * 95).div_ceil(100) - 1),
        max_us: microseconds(ordered.len() - 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_samples_have_no_summary() {
        assert_eq!(summarize(&[]), None);
    }

    #[test]
    fn sorts_samples_and_uses_nearest_rank_percentiles() {
        let samples: Vec<_> = (1..=20).rev().map(Duration::from_millis).collect();
        assert_eq!(
            summarize(&samples),
            Some(Summary {
                samples: 20,
                min_us: 1000.0,
                p50_us: 10000.0,
                p95_us: 19000.0,
                max_us: 20000.0,
            })
        );
    }

    #[test]
    fn one_zero_duration_sample_is_valid() {
        assert_eq!(
            summarize(&[Duration::ZERO]),
            Some(Summary {
                samples: 1,
                min_us: 0.0,
                p50_us: 0.0,
                p95_us: 0.0,
                max_us: 0.0,
            })
        );
    }
}
