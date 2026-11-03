use std::collections::HashMap;
use statrs::distribution::{ContinuousCDF, Normal};

use super::optimizer::{gmv_portfolio, pgd_portfolio, OptMethod};
use super::returns::{
    build_cov_matrix, mean_and_variance, portfolio_dollar_variance,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PositionVaR {
    pub ticker: String,
    pub investment: f64,
    pub single_var: f64,
    pub opt_investment: Option<f64>,
    pub post_var: Option<f64>,
    pub invest_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VaRSummary {
    pub positions: Vec<PositionVaR>,
    pub total_wealth: f64,
    pub undiversified_var: f64,
    pub diversified_var: f64,
    pub diversification_benefit: f64,
    pub opt_undiversified_var: Option<f64>,
    pub opt_diversified_var: Option<f64>,
    pub opt_diversification_benefit: Option<f64>,
    pub var_reduction: Option<f64>,
    pub opt_method_label: Option<String>,
}

pub fn calculate_alpha(confidence_level: f64) -> f64 {
    let n = Normal::new(0.0, 1.0).unwrap_or_else(|_| panic!("Invalid normal distribution parameters"));
    return n.inverse_cdf(confidence_level);
}

pub fn compute_portfolio_var(
    positions: &[(String, f64)],
    returns: &HashMap<String, Vec<f64>>,
    days: f64,
    confidence_level: f64,
    opt_method: Option<OptMethod>,
) -> Result<VaRSummary, String> {
    if positions.is_empty() {
        return Err("Portfolio is empty".to_string());
    }

    let alpha = calculate_alpha(confidence_level);
    let n = positions.len();

    // Check that all positions exist in returns map and have matching non-empty lengths
    let first_len = match returns.get(&positions[0].0) {
        Some(v) if !v.is_empty() => v.len(),
        _ => {
            return Err(format!("No return data available for {}", positions[0].0));
        }
    };

    for (ticker, _) in positions {
        let v = match returns.get(ticker) {
            Some(v) => v,
            None => {
                return Err(format!("Missing return data for ticker {ticker}"));
            }
        };
        if v.len() != first_len {
            return Err(format!(
                "Return length mismatch for {ticker} (expected {first_len}, got {})",
                v.len()
            ));
        }
    }

    let tickers: Vec<String> = positions.iter().map(|p| p.0.clone()).collect();
    let investments: Vec<f64> = positions.iter().map(|p| p.1).collect();
    let total_wealth: f64 = investments.iter().sum();

    // Mean and variance for each asset
    let mut means = Vec::with_capacity(n);
    let mut variances = Vec::with_capacity(n);
    for ticker in &tickers {
        let (mean, var) = mean_and_variance(&returns[ticker]);
        means.push(mean);
        variances.push(var);
    }

    let cov_matrix = build_cov_matrix(&tickers, returns, &means);

    // Baseline VaR
    let current_dollar_var = portfolio_dollar_variance(&investments, &cov_matrix);
    let portfolio_var = alpha * current_dollar_var.max(0.0).sqrt() * days.sqrt();

    // Optimization if selected
    let opt_result: Option<(Vec<f64>, String)> = match opt_method {
        Some(OptMethod::Gmv) => {
            let weights = gmv_portfolio(&cov_matrix, n, total_wealth)?;
            Some((weights, "GMV (Global Min Variance)".to_string()))
        }
        Some(OptMethod::Pgd) => {
            let weights = pgd_portfolio(&cov_matrix, n, total_wealth);
            Some((weights, "PGD (Long-Only Min Variance)".to_string()))
        }
        None => None,
    };

    let mut undiversified_var = 0.0_f64;
    let mut opt_undiversified_var = 0.0_f64;
    let mut position_vars = Vec::with_capacity(n);

    for i in 0..n {
        let sigma = variances[i].max(0.0).sqrt();
        let pre_var = alpha * sigma * investments[i] * days.sqrt();
        undiversified_var += pre_var;

        let (opt_w, post_var, change) = if let Some((ref opt_weights, _)) = opt_result {
            let w = opt_weights[i];
            let post = alpha * sigma * w.abs() * days.sqrt();
            opt_undiversified_var += post;
            (Some(w), Some(post), Some(w - investments[i]))
        } else {
            (None, None, None)
        };

        position_vars.push(PositionVaR {
            ticker: tickers[i].clone(),
            investment: investments[i],
            single_var: pre_var,
            opt_investment: opt_w,
            post_var,
            invest_change: change,
        });
    }

    let diversification_benefit = undiversified_var - portfolio_var;

    let (opt_div_var, opt_undiv_var, opt_div_benefit, var_reduction, method_label) =
        if let Some((ref opt_weights, ref label)) = opt_result {
            let opt_dollar_var = portfolio_dollar_variance(opt_weights, &cov_matrix);
            let opt_portfolio_var = alpha * opt_dollar_var.max(0.0).sqrt() * days.sqrt();
            (
                Some(opt_portfolio_var),
                Some(opt_undiversified_var),
                Some(opt_undiversified_var - opt_portfolio_var),
                Some(portfolio_var - opt_portfolio_var),
                Some(label.clone()),
            )
        } else {
            (None, None, None, None, None)
        };

    return Ok(VaRSummary {
        positions: position_vars,
        total_wealth,
        undiversified_var,
        diversified_var: portfolio_var,
        diversification_benefit,
        opt_undiversified_var: opt_undiv_var,
        opt_diversified_var: opt_div_var,
        opt_diversification_benefit: opt_div_benefit,
        var_reduction,
        opt_method_label: method_label,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_values() {
        let a95 = calculate_alpha(0.95);
        let a99 = calculate_alpha(0.99);
        assert!((a95 - 1.64485).abs() < 1e-4);
        assert!((a99 - 2.32635).abs() < 1e-4);
    }

    #[test]
    fn test_compute_portfolio_var_diversification_benefit() {
        let mut returns = HashMap::new();
        // Asset A and Asset B are negatively correlated
        returns.insert("A".to_string(), vec![0.02, -0.01, 0.03, -0.02, 0.01]);
        returns.insert("B".to_string(), vec![-0.02, 0.01, -0.03, 0.02, -0.01]);

        let positions = vec![("A".to_string(), 50_000.0), ("B".to_string(), 50_000.0)];
        let res = compute_portfolio_var(&positions, &returns, 1.0, 0.99, Some(OptMethod::Pgd))
            .expect("computation should succeed");

        // Diversification benefit must be strictly positive
        assert!(res.diversification_benefit > 0.0);
        assert!(res.diversified_var < res.undiversified_var);

        // Check PGD constraint: non-negative and sums to 100,000
        let sum_opt: f64 = res.positions.iter().map(|p| p.opt_investment.unwrap()).sum();
        assert!((sum_opt - 100_000.0).abs() < 1e-4);
        for p in &res.positions {
            assert!(p.opt_investment.unwrap() >= 0.0);
        }
    }
}
