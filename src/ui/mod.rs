pub mod app_view;
pub mod controls;
pub mod portfolio_table;
pub mod results_view;
pub mod ticker_search;

/// Format dollar values with commas and 2 decimals (e.g. $100,000.00).
pub fn fmt_money(val: f64) -> String {
    let sign = if val < 0.0 { "-" } else { "" };
    let abs = val.abs();
    let int_part = abs.trunc() as u64;
    let frac_part = ((abs - int_part as f64) * 100.0).round() as u64;

    let s = int_part.to_string();
    let mut with_commas = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, &c) in chars.iter().enumerate() {
        with_commas.push(c);
        let remaining = len - 1 - i;
        if remaining > 0 && remaining.is_multiple_of(3) {
            with_commas.push(',');
        }
    }
    return format!("{sign}${with_commas}.{frac_part:02}");
}

/// Format dollar values with commas and no decimals (e.g. $100,000).
pub fn fmt_money_int(val: f64) -> String {
    let sign = if val < 0.0 { "-" } else { "" };
    let abs = val.abs();
    let int_part = abs.round() as u64;

    let s = int_part.to_string();
    let mut with_commas = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, &c) in chars.iter().enumerate() {
        with_commas.push(c);
        let remaining = len - 1 - i;
        if remaining > 0 && remaining.is_multiple_of(3) {
            with_commas.push(',');
        }
    }
    return format!("{sign}${with_commas}");
}

/// Format signed dollar change with commas (e.g. +$10,000).
pub fn fmt_delta(val: f64) -> String {
    let sign = if val >= 0.0 { "+" } else { "-" };
    let abs = val.abs();
    let int_part = abs.round() as u64;

    let s = int_part.to_string();
    let mut with_commas = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, &c) in chars.iter().enumerate() {
        with_commas.push(c);
        let remaining = len - 1 - i;
        if remaining > 0 && remaining.is_multiple_of(3) {
            with_commas.push(',');
        }
    }
    return format!("{sign}${with_commas}");
}

#[cfg(test)]
mod tests {
    use super::app_view::{get_current_search_token, normalize_pasted_text};
    use super::*;

    #[test]
    fn test_normalize_pasted_text() {
        assert_eq!(normalize_pasted_text("aapl, msft"), "AAPL, MSFT");
        assert_eq!(normalize_pasted_text("aapl\nmsft\ntsla"), "AAPL, MSFT, TSLA");
        assert_eq!(normalize_pasted_text("aapl\tmsft\tv"), "AAPL, MSFT, V");
        assert_eq!(normalize_pasted_text("  NVDA \r\n AMZN  "), "NVDA, AMZN");
    }

    #[test]
    fn test_get_current_search_token() {
        assert_eq!(get_current_search_token(""), ("", ""));
        assert_eq!(get_current_search_token("AAPL"), ("", "AAPL"));
        assert_eq!(get_current_search_token("AAPL, MSFT"), ("AAPL,", "MSFT"));
        assert_eq!(get_current_search_token("AAPL, MSFT, TS"), ("AAPL, MSFT,", "TS"));
        assert_eq!(get_current_search_token("AAPL, MSFT, "), ("AAPL, MSFT,", ""));
        assert_eq!(get_current_search_token("AAPL;MSFT;NVDA"), ("AAPL;MSFT;", "NVDA"));
    }

    #[test]
    fn test_fmt_money() {
        assert_eq!(fmt_money(100000.0), "$100,000.00");
        assert_eq!(fmt_money(1234567.89), "$1,234,567.89");
        assert_eq!(fmt_money(-500.5), "-$500.50");
    }

    #[test]
    fn test_user_large_paste_all_cached() {
        use crate::risk::find_cached_csv;
        let paste = "AAPL, MSFT, GOOGL, AMZN, NVDA, META, BRK.B, TSLA, AVGO, LLY, JPM, V, XOM, UNH, MA, COST, HD, WMT, PG, JNJ, ABBV, BAC, NFLX, CRM, CVX, AMD, MRK, TMUS, LIN, KO, PLTR, WFC, DIS, RTX, ACN, PM, CSCO, QCOM, TMO, IBM, GE, INTU, CAT, AMAT, DHR, BA, BKNG, SPGI, ISRG, TXN, NOW, HON, UPS, PFE, LOW, VRTX, COP, SBUX, BLK, DE, ADP, TJX, MDT, MDT, GILD, CB, C, LMT, PANW, T, MU, REGN, MO,ADI, CMCSA, SHW, ETN, ICE, BNY, CL, MMM, EMR, CDNS, HUM, NSC, SLB, ADSK, ITW, PSA, CME, MCD, HWM, ORCL, PCAR, ADBE, MCO, WM, PH";
        let mut raw_tickers: Vec<String> = Vec::new();
        for part in paste.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
            let clean = part.trim().to_uppercase().replace('.', "-");
            if !clean.is_empty() && !raw_tickers.contains(&clean) {
                raw_tickers.push(clean);
            }
        }

        assert_eq!(raw_tickers.len(), 97);
        for ticker in &raw_tickers {
            assert!(
                find_cached_csv(ticker).is_some(),
                "Expected to find cached CSV for '{ticker}'"
            );
        }
    }

    #[test]
    fn test_97_tickers_compute_portfolio_var_fast() {
        use crate::risk::{
            compute_portfolio_var, find_cached_csv, load_prices, log_returns, OptMethod,
        };
        use std::collections::HashMap;

        let paste = "AAPL, MSFT, GOOGL, AMZN, NVDA, META, BRK.B, TSLA, AVGO, LLY, JPM, V, XOM, UNH, MA, COST, HD, WMT, PG, JNJ, ABBV, BAC, NFLX, CRM, CVX, AMD, MRK, TMUS, LIN, KO, PLTR, WFC, DIS, RTX, ACN, PM, CSCO, QCOM, TMO, IBM, GE, INTU, CAT, AMAT, DHR, BA, BKNG, SPGI, ISRG, TXN, NOW, HON, UPS, PFE, LOW, VRTX, COP, SBUX, BLK, DE, ADP, TJX, MDT, MDT, GILD, CB, C, LMT, PANW, T, MU, REGN, MO,ADI, CMCSA, SHW, ETN, ICE, BNY, CL, MMM, EMR, CDNS, HUM, NSC, SLB, ADSK, ITW, PSA, CME, MCD, HWM, ORCL, PCAR, ADBE, MCO, WM, PH";
        let mut raw_tickers: Vec<String> = Vec::new();
        for part in paste.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
            let clean = part.trim().to_uppercase().replace('.', "-");
            if !clean.is_empty() && !raw_tickers.contains(&clean) {
                raw_tickers.push(clean);
            }
        }

        let mut returns_map: HashMap<String, Vec<f64>> = HashMap::new();
        let mut min_len = usize::MAX;
        let mut positions = Vec::new();

        for ticker in &raw_tickers {
            let path = find_cached_csv(ticker).unwrap();
            let prices = load_prices(&path, Some(500)).unwrap();
            let r = log_returns(&prices);
            if r.len() < min_len {
                min_len = r.len();
            }
            returns_map.insert(ticker.clone(), r);
            positions.push((ticker.clone(), 10_000.0));
        }

        let mut aligned_returns: HashMap<String, Vec<f64>> = HashMap::new();
        for ticker in &raw_tickers {
            let r = &returns_map[ticker];
            let start = r.len() - min_len;
            aligned_returns.insert(ticker.clone(), r[start..].to_vec());
        }

        let start_time = std::time::Instant::now();
        let summary_pgd = compute_portfolio_var(
            &positions,
            &aligned_returns,
            5.0,
            0.99,
            Some(OptMethod::Pgd),
        )
        .expect("PGD optimization should succeed on 97 tickers");

        let duration = start_time.elapsed();
        println!("97 assets PGD optimization took: {:?}", duration);

        assert_eq!(summary_pgd.positions.len(), 97);
        assert!(summary_pgd.opt_diversified_var.is_some());
        assert!(summary_pgd.var_reduction.unwrap() >= 0.0);
    }
}
