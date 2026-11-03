use std::collections::HashMap;
use crate::risk::{
    compute_portfolio_var, find_cached_csv, load_prices, log_returns, OptMethod, VaRSummary,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PositionItem {
    pub ticker: String,
    pub name: String,
    pub investment: f64,
    pub investment_input: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub positions: Vec<PositionItem>,
    pub returns_cache: HashMap<String, Vec<f64>>,
    pub days: f64,
    pub confidence_level: f64,
    pub opt_method: Option<OptMethod>,
    pub summary: Option<VaRSummary>,
    pub status: String,
    pub search_query: String,
    pub is_busy: bool,
    pub is_calculating: bool,
    pub active_tab: Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Portfolio,
    Results,
}

impl Default for AppState {
    fn default() -> Self {
        return Self {
            positions: Vec::new(),
            returns_cache: HashMap::new(),
            days: 5.0,
            confidence_level: 0.99,
            opt_method: Some(OptMethod::Pgd),
            summary: None,
            status: "Ready. Add tickers to begin risk analysis.".to_string(),
            search_query: String::new(),
            is_busy: false,
            is_calculating: false,
            active_tab: Tab::Portfolio,
        };
    }
}

impl AppState {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn total_wealth(&self) -> f64 {
        return self.positions.iter().map(|p| p.investment).sum();
    }

    pub fn has_ticker(&self, ticker: &str) -> bool {
        let t = ticker.to_uppercase();
        return self.positions.iter().any(|p| p.ticker == t);
    }

    /// Update position data in-place without triggering synchronous VaR recalculation.
    pub fn add_or_update_position_silent(
        &mut self,
        ticker: String,
        name: String,
        investment: f64,
        prices: Vec<f64>,
    ) {
        let ticker_upper = ticker.to_uppercase();
        let returns = log_returns(&prices);
        self.returns_cache.insert(ticker_upper.clone(), returns);

        if let Some(pos) = self.positions.iter_mut().find(|p| p.ticker == ticker_upper) {
            pos.investment = investment;
            pos.investment_input = format!("{investment:.0}");
            pos.name = name;
        } else {
            self.positions.push(PositionItem {
                ticker: ticker_upper,
                name,
                investment,
                investment_input: format!("{investment:.0}"),
            });
        }
    }

    pub fn add_or_update_position(
        &mut self,
        ticker: String,
        name: String,
        investment: f64,
        prices: Vec<f64>,
    ) {
        self.add_or_update_position_silent(ticker, name, investment, prices);
        self.recalculate_var();
    }

    /// Remove position silently without immediate synchronous recalculation.
    pub fn remove_position_silent(&mut self, ticker: &str) {
        let t = ticker.to_uppercase();
        self.positions.retain(|p| p.ticker != t);
        self.returns_cache.remove(&t);
    }

    pub fn remove_position(&mut self, ticker: &str) {
        self.remove_position_silent(ticker);
        self.recalculate_var();
    }

    /// Remove all positions silently from state.
    pub fn clear_all_positions_silent(&mut self) {
        self.positions.clear();
        self.returns_cache.clear();
        self.summary = None;
    }

    /// Reset all positions to equal investment amounts based on existing total wealth.
    /// If total wealth is zero, defaults each asset to $100,000.
    pub fn reset_equal_weights_silent(&mut self) {
        let n = self.positions.len();
        if n == 0 {
            return;
        }
        let total = self.total_wealth();
        let target_total = if total > 1e-4 {
            total
        } else {
            100_000.0 * n as f64
        };
        let per_asset = target_total / n as f64;
        for pos in &mut self.positions {
            pos.investment = per_asset;
            pos.investment_input = format!("{per_asset:.0}");
        }
    }

    /// Update investment amount silently without immediate synchronous recalculation.
    pub fn update_investment_silent(&mut self, ticker: &str, amount: f64) {
        let t = ticker.to_uppercase();
        if let Some(pos) = self.positions.iter_mut().find(|p| p.ticker == t) {
            pos.investment = amount.max(0.0);
            pos.investment_input = format!("{amount:.0}");
        }
    }

    pub fn update_investment(&mut self, ticker: &str, amount: f64) {
        self.update_investment_silent(ticker, amount);
        self.recalculate_var();
    }

    /// Rescale all positions proportionally to match a target total wealth amount.
    /// If prior total wealth was zero, distributes wealth equally across all positions.
    pub fn scale_to_total_wealth_silent(&mut self, new_total: f64) {
        let n = self.positions.len();
        if n == 0 {
            return;
        }
        let old_total = self.total_wealth();
        let target = new_total.max(0.0);

        if old_total > 1e-6 {
            let factor = target / old_total;
            for pos in &mut self.positions {
                let updated = (pos.investment * factor).max(0.0);
                pos.investment = updated;
                pos.investment_input = format!("{updated:.0}");
            }
        } else {
            let per_asset = target / n as f64;
            for pos in &mut self.positions {
                pos.investment = per_asset;
                pos.investment_input = format!("{per_asset:.0}");
            }
        }
    }

    pub fn scale_to_total_wealth(&mut self, new_total: f64) {
        self.scale_to_total_wealth_silent(new_total);
        self.recalculate_var();
    }

    /// Align return vectors to common length (shortest available) and recalculate VaR.
    pub fn recalculate_var(&mut self) {
        if self.positions.is_empty() {
            self.summary = None;
            return;
        }

        // Determine shortest returns length across all present positions
        let mut min_len = usize::MAX;
        for pos in &self.positions {
            if let Some(r) = self.returns_cache.get(&pos.ticker) {
                if !r.is_empty() && r.len() < min_len {
                    min_len = r.len();
                }
            } else {
                // If returns are not loaded for a position, try to load from cached file in ./data/
                if let Some(cached_path) = find_cached_csv(&pos.ticker)
                    && let Ok(prices) = load_prices(&cached_path, Some(500))
                {
                    let r = log_returns(&prices);
                    if !r.is_empty() && r.len() < min_len {
                        min_len = r.len();
                    }
                    self.returns_cache.insert(pos.ticker.clone(), r);
                    continue;
                }
                self.summary = None;
                self.status = format!("Missing cached prices for {}", pos.ticker);
                return;
            }
        }

        if min_len == usize::MAX || min_len < 2 {
            self.summary = None;
            self.status = "Insufficient historical price points to compute VaR".to_string();
            return;
        }

        // Build slice with aligned length
        let mut aligned_returns: HashMap<String, Vec<f64>> = HashMap::new();
        for pos in &self.positions {
            if let Some(r) = self.returns_cache.get(&pos.ticker) {
                let start_idx = r.len() - min_len;
                aligned_returns.insert(pos.ticker.clone(), r[start_idx..].to_vec());
            }
        }

        let positions_tuples: Vec<(String, f64)> = self
            .positions
            .iter()
            .map(|p| (p.ticker.clone(), p.investment))
            .collect();

        match compute_portfolio_var(
            &positions_tuples,
            &aligned_returns,
            self.days,
            self.confidence_level,
            self.opt_method,
        ) {
            Ok(summary) => {
                self.summary = Some(summary);
                self.status = format!(
                    "VaR computed: Total Wealth ${:.2}, {} positions",
                    self.total_wealth(),
                    self.positions.len()
                );
            }
            Err(e) => {
                self.summary = None;
                self.status = format!("Calculation error: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_to_total_wealth_proportional() {
        let mut state = AppState::new();
        state.positions.push(PositionItem {
            ticker: "AAPL".to_string(),
            name: "Apple".to_string(),
            investment: 100_000.0,
            investment_input: "100000".to_string(),
        });
        state.positions.push(PositionItem {
            ticker: "MSFT".to_string(),
            name: "Microsoft".to_string(),
            investment: 300_000.0,
            investment_input: "300000".to_string(),
        });

        assert_eq!(state.total_wealth(), 400_000.0);

        // Scale to $1,000,000 (factor 2.5)
        state.scale_to_total_wealth_silent(1_000_000.0);

        assert_eq!(state.total_wealth(), 1_000_000.0);
        assert_eq!(state.positions[0].investment, 250_000.0);
        assert_eq!(state.positions[1].investment, 750_000.0);
    }

    #[test]
    fn test_scale_to_total_wealth_from_zero() {
        let mut state = AppState::new();
        state.positions.push(PositionItem {
            ticker: "AAPL".to_string(),
            name: "Apple".to_string(),
            investment: 0.0,
            investment_input: "0".to_string(),
        });
        state.positions.push(PositionItem {
            ticker: "MSFT".to_string(),
            name: "Microsoft".to_string(),
            investment: 0.0,
            investment_input: "0".to_string(),
        });

        assert_eq!(state.total_wealth(), 0.0);

        // Scale to $500,000 (equal split $250k each)
        state.scale_to_total_wealth_silent(500_000.0);

        assert_eq!(state.total_wealth(), 500_000.0);
        assert_eq!(state.positions[0].investment, 250_000.0);
        assert_eq!(state.positions[1].investment, 250_000.0);
    }

    #[test]
    fn test_reset_equal_weights() {
        let mut state = AppState::new();
        state.positions.push(PositionItem {
            ticker: "AAPL".to_string(),
            name: "Apple".to_string(),
            investment: 10_000.0,
            investment_input: "10000".to_string(),
        });
        state.positions.push(PositionItem {
            ticker: "MSFT".to_string(),
            name: "Microsoft".to_string(),
            investment: 90_000.0,
            investment_input: "90000".to_string(),
        });

        assert_eq!(state.total_wealth(), 100_000.0);

        state.reset_equal_weights_silent();

        assert_eq!(state.total_wealth(), 100_000.0);
        assert_eq!(state.positions[0].investment, 50_000.0);
        assert_eq!(state.positions[1].investment, 50_000.0);
    }

    #[test]
    fn test_clear_all_positions() {
        let mut state = AppState::new();
        state.positions.push(PositionItem {
            ticker: "AAPL".to_string(),
            name: "Apple".to_string(),
            investment: 10_000.0,
            investment_input: "10000".to_string(),
        });

        assert_eq!(state.positions.len(), 1);
        state.clear_all_positions_silent();
        assert!(state.positions.is_empty());
        assert_eq!(state.total_wealth(), 0.0);
    }
}
