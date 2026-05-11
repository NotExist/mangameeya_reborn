use serde::{Deserialize, Serialize};

/// Structured bench output, dumped to stdout as JSON for downstream tooling.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct BenchReport {
    pub version: String,
    pub platform: String,
    pub scenario: String,
    pub fixture: FixtureInfo,
    pub timings: Timings,
    pub resource: ResourceUsage,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FixtureInfo {
    pub path: String,
    pub size_bytes: u64,
    pub page_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Timings {
    pub zip_open_ms: f64,
    pub first_page_decode_ms: f64,
    pub startup_to_first_page_ms: f64,
    pub cold_decode_p50_ms: f64,
    pub cold_decode_p95_ms: f64,
    pub cold_decode_p99_ms: f64,
    pub cold_decode_max_ms: f64,
    pub resize_lanczos_p50_ms: f64,
    pub resize_lanczos_p99_ms: f64,
    pub parallel_preload_5_pages_ms: f64,
    pub samples: usize,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ResourceUsage {
    pub idle_cpu_pct: f64,
    pub working_set_after_5_pages_mb: f64,
    pub peak_rss_mb: f64,
}

/// Nearest-rank percentile (1-indexed), clamped to `[0, n-1]`.
///
/// For `n = 100`, `p = 50` returns `sorted[49]`.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    let raw = (p / 100.0 * n as f64).ceil() as usize;
    let idx = raw.saturating_sub(1).min(n - 1);
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_basic() {
        let v: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        assert_eq!(percentile(&v, 50.0), 50.0);
        assert_eq!(percentile(&v, 99.0), 99.0);
        assert_eq!(percentile(&v, 100.0), 100.0);
        assert_eq!(percentile(&v, 0.0), 1.0);
    }

    #[test]
    fn percentile_empty() {
        let v: Vec<f64> = vec![];
        assert_eq!(percentile(&v, 50.0), 0.0);
    }
}
