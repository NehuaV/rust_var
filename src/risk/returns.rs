use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Returns today's UTC date as "YYYY-MM-DD" using only `std::time`.
/// Uses Hinnant's civil_from_days algorithm.
pub fn today_ymd() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86400;

    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    return format!("{y:04}-{m:02}-{d:02}");
}

/// Find the freshest, most relevant cached CSV file for a given ticker in `./data/`.
/// Checks patterns:
/// - `500d_{TICKER}_{DATE}.csv`
/// - `1y_{TICKER}_{DATE}.csv`
/// - `500d_{TICKER}.csv`
/// - `1y_{TICKER}.csv`
/// - `{TICKER}.csv`
///
/// Prioritizes files containing today's date, then by latest date string in the filename,
/// then by file modification time (mtime), preferring 500d files.
pub fn find_cached_csv(ticker: &str) -> Option<PathBuf> {
    return find_cached_csv_in_dir(ticker, Path::new("./data"));
}

/// Directory-parameterized search helper for testability and custom paths.
pub fn find_cached_csv_in_dir(ticker: &str, dir: &Path) -> Option<PathBuf> {
    let t_upper = ticker.trim().to_uppercase();
    if t_upper.is_empty() || !dir.exists() {
        return None;
    }
    let t_hyphen = t_upper.replace('.', "-");
    let t_dot = t_upper.replace('-', ".");

    let entries = fs::read_dir(dir).ok()?;
    let mut matches: Vec<(PathBuf, bool, String, u64)> = Vec::new();
    let today = today_ymd();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("csv") {
            continue;
        }

        let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };

        // Split stem by underscore
        let parts: Vec<&str> = file_stem.split('_').collect();
        // Check if ticker is an exact match for one of the parts (testing upper, hyphen, and dot variants)
        let is_ticker_match = parts.iter().any(|&p| {
            p.eq_ignore_ascii_case(&t_upper)
                || p.eq_ignore_ascii_case(&t_hyphen)
                || p.eq_ignore_ascii_case(&t_dot)
        });
        if !is_ticker_match {
            continue;
        }

        // Check if file contains today's date
        let is_today = file_stem.contains(&today);
        let is_500d = file_stem.starts_with("500d") || file_stem.starts_with("2y");

        // Extract any date pattern (YYYY-MM-DD) if present
        let date_part = parts
            .iter()
            .find(|p| p.len() == 10 && p.chars().nth(4) == Some('-') && p.chars().nth(7) == Some('-'))
            .unwrap_or(&"")
            .to_string();

        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Boost 500d files slightly if multiple files have the same date
        let score = if is_500d { mtime + 10_000 } else { mtime };
        matches.push((path, is_today, date_part, score));
    }

    if matches.is_empty() {
        return None;
    }

    // Sort matches:
    // 1. Files for today come first
    // 2. Later date_part string comes first
    // 3. Higher mtime score comes first
    matches.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| b.3.cmp(&a.3))
    });

    return Some(matches.remove(0).0);
}

/// Load prices from a 2-column CSV (Date, Close).
/// Trims to the most recent `max_days` if specified (e.g. 500).
pub fn load_prices<P: AsRef<Path>>(path: P, max_days: Option<usize>) -> io::Result<Vec<f64>> {
    let p = path.as_ref();
    let content = match fs::read_to_string(p) {
        Ok(c) => c,
        Err(e) => {
            return Err(e);
        }
    };

    let mut prices: Vec<f64> = content
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let comma = line.find(',')?;
            line[comma + 1..].trim().parse::<f64>().ok()
        })
        .collect();

    if prices.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("No valid prices found in '{}'", p.display()),
        ));
    }

    if let Some(limit) = max_days
        && prices.len() > limit
    {
        let start = prices.len() - limit;
        prices = prices[start..].to_vec();
    }

    return Ok(prices);
}

/// Log returns: r_t = ln(P_t / P_{t-1})
pub fn log_returns(prices: &[f64]) -> Vec<f64> {
    return prices.windows(2).map(|w| (w[1] / w[0]).ln()).collect();
}

/// Single-pass mean + sample variance via Welford's online algorithm
pub fn mean_and_variance(v: &[f64]) -> (f64, f64) {
    if v.len() < 2 {
        return (v.first().copied().unwrap_or(0.0), 0.0);
    }

    let mut mean: f64 = 0.0;
    let mut m2: f64 = 0.0;

    for (i, &x) in v.iter().enumerate() {
        let delta = x - mean;
        mean += delta / (i + 1) as f64;
        m2 += (x - mean) * delta;
    }

    return (mean, m2 / (v.len() - 1) as f64);
}

/// Sample covariance: Cov(R_i, R_j) = Σ(R_i − μ_i)(R_j − μ_j) / (N − 1)
pub fn vec_covariance(a: &[f64], mu_a: f64, b: &[f64], mu_b: f64) -> f64 {
    let n = a.len().min(b.len());
    if n < 2 {
        return 0.0;
    }
    let cov: f64 = a[..n]
        .iter()
        .zip(&b[..n])
        .map(|(&ai, &bi)| (ai - mu_a) * (bi - mu_b))
        .sum();

    return cov / (n - 1) as f64;
}

/// NxN variance-covariance matrix as a flat row-major Vec.
/// Uses Rayon to parallelize row computations across available CPU cores.
pub fn build_cov_matrix(
    portfolio_tickers: &[String],
    returns: &HashMap<String, Vec<f64>>,
    means: &[f64],
) -> Vec<f64> {
    let n = portfolio_tickers.len();
    let mut sigma = vec![0.0_f64; n * n];

    sigma
        .par_chunks_mut(n)
        .enumerate()
        .for_each(|(i, row)| {
            let ret_i = &returns[&portfolio_tickers[i]];
            let mean_i = means[i];
            for j in 0..n {
                let ret_j = &returns[&portfolio_tickers[j]];
                let mean_j = means[j];
                row[j] = vec_covariance(ret_i, mean_i, ret_j, mean_j);
            }
        });

    return sigma;
}

/// Portfolio dollar variance: x' · Σ · x  (flat row-major Σ)
pub fn portfolio_dollar_variance(investments: &[f64], sigma: &[f64]) -> f64 {
    let n = investments.len();
    let mut total: f64 = 0.0;

    for i in 0..n {
        for j in 0..n {
            total += investments[i] * investments[j] * sigma[i * n + j];
        }
    }

    return total;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_find_cached_csv_exact_and_isolation() {
        let tmp_dir = std::env::temp_dir().join("rust_var_test_cache");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();

        // Create test dummy files
        File::create(tmp_dir.join("1y_AAPL_2026-07-06.csv")).unwrap();
        File::create(tmp_dir.join("1y_AAPL_2026-11-03.csv")).unwrap();
        File::create(tmp_dir.join("1y_V_2026-07-13.csv")).unwrap();
        File::create(tmp_dir.join("1y_VRTX_2026-07-13.csv")).unwrap();

        // AAPL should find the newest 2026-11-03 file
        let aapl_match = find_cached_csv_in_dir("AAPL", &tmp_dir).unwrap();
        assert!(aapl_match.to_string_lossy().contains("1y_AAPL_2026-11-03.csv"));

        // V should find V file and NOT VRTX
        let v_match = find_cached_csv_in_dir("V", &tmp_dir).unwrap();
        assert!(v_match.to_string_lossy().contains("1y_V_2026-07-13.csv"));

        // VRTX should find VRTX file
        let vrtx_match = find_cached_csv_in_dir("VRTX", &tmp_dir).unwrap();
        assert!(vrtx_match.to_string_lossy().contains("1y_VRTX_2026-07-13.csv"));

        // Non-existent ticker should return None
        assert!(find_cached_csv_in_dir("NONEXISTENT", &tmp_dir).is_none());

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_load_prices_trim() {
        let tmp_file = std::env::temp_dir().join("test_prices_trim.csv");
        let mut f = File::create(&tmp_file).unwrap();
        writeln!(f, "Date,Price").unwrap();
        for i in 1..=600 {
            writeln!(f, "2024-01-01,{i}.0").unwrap();
        }

        // Test without limit
        let all = load_prices(&tmp_file, None).unwrap();
        assert_eq!(all.len(), 600);

        // Test with max_days = 500
        let trimmed = load_prices(&tmp_file, Some(500)).unwrap();
        assert_eq!(trimmed.len(), 500);
        assert_eq!(trimmed[0], 101.0);
        assert_eq!(trimmed[499], 600.0);

        let _ = fs::remove_file(&tmp_file);
    }

    #[test]
    fn test_real_cached_files_exist_and_load() {
        for ticker in &["AAPL", "MSFT", "TSLA", "V", "BRK.B", "BRK-B"] {
            let found = find_cached_csv(ticker);
            assert!(
                found.is_some(),
                "Expected to find cached CSV file for ticker {ticker}"
            );
            let prices = load_prices(&found.unwrap(), Some(500)).unwrap();
            assert!(
                !prices.is_empty(),
                "Expected non-empty prices for ticker {ticker}"
            );
        }
    }
}
