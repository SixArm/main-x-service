//! Salary benchmarking flags (WPM-R14), DB-free.

/// The comparison verdict for one employee against a benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkFlag {
    /// Below the recorded market minimum.
    BelowMin,
    /// Within the recorded band.
    Within,
    /// Above the recorded market maximum.
    AboveMax,
}

/// Compare a salary to a benchmark band. `None` when the currencies
/// differ — mixed currencies never silently compare (WPM-D4 family
/// posture).
#[must_use]
pub fn compare(
    salary_minor: i64,
    salary_currency: &str,
    band_min: i64,
    band_max: i64,
    band_currency: &str,
) -> Option<BenchmarkFlag> {
    if !salary_currency.eq_ignore_ascii_case(band_currency) {
        return None;
    }
    Some(if salary_minor < band_min {
        BenchmarkFlag::BelowMin
    } else if salary_minor > band_max {
        BenchmarkFlag::AboveMax
    } else {
        BenchmarkFlag::Within
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Below / within / above / boundary values flag correctly, and a
    /// currency mismatch refuses to compare.
    #[test]
    fn flags_and_currency_guard() {
        assert_eq!(compare(90, "GBP", 100, 200, "GBP"), Some(BenchmarkFlag::BelowMin));
        assert_eq!(compare(100, "GBP", 100, 200, "GBP"), Some(BenchmarkFlag::Within));
        assert_eq!(compare(200, "GBP", 100, 200, "gbp"), Some(BenchmarkFlag::Within));
        assert_eq!(compare(201, "GBP", 100, 200, "GBP"), Some(BenchmarkFlag::AboveMax));
        assert_eq!(compare(150, "USD", 100, 200, "GBP"), None);
    }
}
