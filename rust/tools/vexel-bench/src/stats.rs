//! Order statistics over millisecond samples.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Summary {
    pub n: usize,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
    pub mad: f64,
}

fn sorted(values: &[f64]) -> Vec<f64> {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    v
}

fn median_sorted(v: &[f64]) -> f64 {
    match v.len() {
        0 => 0.0,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// Nearest-rank percentile of an ascending slice.
fn rank(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let idx = ((p * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1;
    v[idx]
}

pub fn summarize(samples: &[f64]) -> Summary {
    let v = sorted(samples);
    let p50 = median_sorted(&v);
    let deviations: Vec<f64> = v.iter().map(|x| (x - p50).abs()).collect();
    Summary {
        n: v.len(),
        p50,
        p95: rank(&v, 0.95),
        p99: rank(&v, 0.99),
        max: v.last().copied().unwrap_or(0.0),
        mad: median_sorted(&sorted(&deviations)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_of_known_samples() {
        let s = summarize(&[5.0, 1.0, 3.0, 2.0, 4.0]);
        assert_eq!((s.n, s.p50, s.max), (5, 3.0, 5.0));
        assert_eq!((s.p95, s.p99), (5.0, 5.0));
        assert_eq!(s.mad, 1.0);
    }

    #[test]
    fn even_count_median_averages_middle_pair() {
        assert_eq!(summarize(&[1.0, 2.0, 3.0, 10.0]).p50, 2.5);
        assert_eq!(summarize(&[]), Summary::default());
    }
}
