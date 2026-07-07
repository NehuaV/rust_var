use std::collections::HashMap;
use std::fs;
use std::io;

use clap::{Parser, ValueEnum};
use nalgebra::{DMatrix, DVector};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Position {
    ticker: &'static str,
    investment: f64,
}

/// Returns today's UTC date as "YYYY-MM-DD" using only `std::time`.
/// Uses Hinnant's civil_from_days algorithm (public domain).
fn today_ymd() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86400;

    // civil_from_days — maps days-since-Unix-epoch to (y, m, d)
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

    format!("{y:04}-{m:02}-{d:02}")
}

fn load_prices(path: &str) -> io::Result<Vec<f64>> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file '{}': {}", path, e);
            return Err(e.into());
        }
    };

    let prices = content
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let comma = line.find(',')?;

            line[comma + 1..].trim().parse::<f64>().ok()
        })
        .collect();

    return Ok(prices);
}

fn log_returns(prices: &[f64]) -> Vec<f64> {
    return prices.windows(2).map(|w| (w[1] / w[0]).ln()).collect();
}

// Single-pass mean + variance via Welford's online algorithm
fn mean_and_variance(v: &[f64]) -> (f64, f64) {
    let mut mean: f64 = 0.0;
    let mut m2: f64 = 0.0;

    for (i, &x) in v.iter().enumerate() {
        let delta = x - mean;
        mean += delta / (i + 1) as f64;
        m2 += (x - mean) * delta;
    }

    return (mean, m2 / (v.len() - 1) as f64);
}

// Cov(R_i, R_j) = Σ(R_i − μ_i)(R_j − μ_j) / (N − 1)
fn vec_covariance(a: &[f64], mu_a: f64, b: &[f64], mu_b: f64) -> f64 {
    let n = a.len().min(b.len());
    let cov: f64 = a[..n]
        .iter()
        .zip(&b[..n])
        .map(|(&ai, &bi)| (ai - mu_a) * (bi - mu_b))
        .sum();

    return cov / (n - 1) as f64;
}

// NxN variance-covariance matrix as a flat row-major Vec.
// Upper triangle computed in parallel; lower triangle filled by symmetry.
fn build_cov_matrix(
    portfolio: &[Position],
    returns: &HashMap<&str, Vec<f64>>,
    means: &[f64],
) -> Vec<f64> {
    let n = portfolio.len();

    let upper: Vec<(usize, usize, f64)> = (0..n)
        .flat_map(|i| (i..n).map(move |j| (i, j)))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(i, j)| {
            let cov = vec_covariance(
                &returns[portfolio[i].ticker],
                means[i],
                &returns[portfolio[j].ticker],
                means[j],
            );
            return (i, j, cov);
        })
        .collect();

    let mut sigma = vec![0.0_f64; n * n];
    for (i, j, cov) in upper {
        sigma[i * n + j] = cov;
        sigma[j * n + i] = cov;
    }
    return sigma;
}

// Portfolio dollar variance: x' · Σ · x  (flat row-major Σ)
fn portfolio_dollar_variance(portfolio: &[Position], sigma: &[f64]) -> f64 {
    let n = portfolio.len();
    let mut total: f64 = 0.0;

    for i in 0..n {
        for j in 0..n {
            total += portfolio[i].investment * portfolio[j].investment * sigma[i * n + j];
        }
    }

    return total;
}

/// Global Minimum Variance (GMV) portfolio via the analytical closed-form solution.
///
/// We want to find the dollar-investment vector **x** that minimises portfolio
/// variance `x' Σ x` subject only to the budget constraint `1'x = W` (short
/// selling is allowed).
///
/// Derivation (Lagrangian):
///   L(x, λ) = x'Σx − λ(1'x − W)
///   ∂L/∂x = 2Σx − λ1 = 0  →  x = (λ/2) Σ⁻¹1
///   Applying the budget constraint 1'x = W:
///   λ/2 = W / (1'Σ⁻¹1)
///
/// Therefore:
///   x* = (Σ⁻¹1 / 1'Σ⁻¹1) · W
///
/// `sigma_flat` is the NxN covariance matrix in row-major order.
/// Returns a Vec of optimal dollar investments in the same order as `portfolio`.
fn gmv_portfolio(sigma_flat: &[f64], n: usize, total_wealth: f64) -> Vec<f64> {
    // Wrap the flat slice into a nalgebra dense matrix (row-major).
    let sigma = DMatrix::from_row_slice(n, n, sigma_flat);

    // Σ⁻¹ — nalgebra uses LU decomposition internally; returns None if singular.
    let sigma_inv = sigma
        .try_inverse()
        .expect("covariance matrix is singular; cannot compute GMV portfolio");

    // Σ⁻¹ · 1  (multiply inverse by the all-ones vector)
    let ones = DVector::from_element(n, 1.0_f64);
    let sigma_inv_ones = &sigma_inv * &ones;

    // Scalar denominator: 1' · Σ⁻¹ · 1
    let denom = ones.dot(&sigma_inv_ones);

    // x* = (Σ⁻¹1 / denom) · W
    let x_star = sigma_inv_ones * (total_wealth / denom);

    return x_star.data.as_vec().clone();
}

/// Projects `v` in-place onto the scaled probability simplex
/// { x : x_i ≥ 0,  Σ x_i = budget } in O(n log n).
///
/// Algorithm: Duchi et al. (2008) "Efficient Projections onto the ℓ₁-Ball
/// for Learning in High Dimensions", ICML.
///
/// The key insight is that the Lagrange multiplier θ that enforces Σx = budget
/// while clipping x to [0,∞) can be found analytically by sorting once and
/// scanning left-to-right for the largest index ρ such that
///   u_ρ > (Σ_{j≤ρ} u_j − budget) / (ρ + 1)
/// where u is the sorted (descending) copy of v.  Then θ = (Σ_{j≤ρ} u_j − budget) / (ρ+1)
/// and x_i = max(v_i − θ, 0).
fn project_simplex(v: &mut [f64], budget: f64) {
    let mut u: Vec<f64> = v.to_vec();
    u.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap()); // descending

    let mut cssv: f64 = 0.0; // cumulative sorted-sum
    let mut rho: usize = 0; // largest index satisfying the condition

    for (j, &uj) in u.iter().enumerate() {
        cssv += uj;
        if uj - (cssv - budget) / (j + 1) as f64 > 0.0 {
            rho = j;
        }
    }

    // Threshold that uniformly shifts all components so the budget is met.
    let theta: f64 = (u[..=rho].iter().sum::<f64>() - budget) / (rho + 1) as f64;

    for xi in v.iter_mut() {
        *xi = (*xi - theta).max(0.0);
    }
}

/// Long-only Global Minimum Variance portfolio via Projected Gradient Descent.
///
/// Solves the constrained QP:
///   min  x' Σ x
///   s.t. x ≥ 0,  Σ x_i = W
///
/// The gradient of the objective is ∇f(x) = 2Σx.  Each iteration:
///   1. Gradient step:  x ← x − lr · 2Σx
///   2. Projection:     x ← project_simplex(x, W)
///
/// Step size lr = 1 / (2 λ_max(Σ)).  We bound λ_max from above using
/// the Gershgorin circle theorem:
///   λ_max ≤ max_i  Σ_j |Σ_ij|   (max absolute row sum)
/// which gives lr = 1 / (2 · ‖Σ‖_∞) — guaranteed convergence, conservative
/// but safe even for ill-conditioned matrices.
///
/// Iteration stops when the sum of absolute weight changes falls below
/// `budget × 1e-9` (tight enough for any financial application) or after
/// 10 000 steps, whichever comes first.
fn pgd_portfolio(sigma: &[f64], n: usize, budget: f64) -> Vec<f64> {
    // ‖Σ‖_∞ = max absolute row sum  →  guaranteed upper bound for λ_max
    let l_inf: f64 = (0..n)
        .map(|i| (0..n).map(|j| sigma[i * n + j].abs()).sum::<f64>())
        .fold(f64::NEG_INFINITY, f64::max);
    let lr = 1.0 / (2.0 * l_inf);

    let mut x = vec![budget / n as f64; n]; // start at equal weights
    let mut grad = vec![0.0_f64; n];

    for _ in 0..10_000 {
        // Compute gradient: grad = 2 · Σ · x
        for gi in grad.iter_mut() {
            *gi = 0.0;
        }
        for i in 0..n {
            for j in 0..n {
                grad[i] += sigma[i * n + j] * x[j];
            }
        }

        // Gradient step
        let mut x_new: Vec<f64> = x
            .iter()
            .zip(&grad)
            .map(|(&xi, &gi)| xi - lr * 2.0 * gi)
            .collect();

        // Project back onto the simplex
        project_simplex(&mut x_new, budget);

        // Check convergence: stop when total weight movement < budget · ε
        let delta: f64 = x_new.iter().zip(&x).map(|(a, b)| (a - b).abs()).sum();
        x = x_new;

        if delta / budget < 1e-9 {
            break;
        }
    }

    return x;
}

#[rustfmt::skip]
const POSITIONS: &[(&str, f64)] = &[
    ("AAPL",  100_000.0), ("MSFT",  100_000.0), ("GOOG",  100_000.0),
    ("GOOGL", 100_000.0), ("META",  100_000.0), ("AMZN",  100_000.0),
    ("TSLA",  100_000.0), ("NVDA",  100_000.0), ("BRK-B", 100_000.0),
    ("JPM",   100_000.0), ("JNJ",   100_000.0), ("V",     100_000.0),
    ("WMT",   100_000.0), ("PG",    100_000.0), ("MA",    100_000.0),
    ("DIS",   100_000.0), ("HD",    100_000.0), ("BAC",   100_000.0),
    ("XOM",   100_000.0), ("CVX",   100_000.0), ("KO",    100_000.0),
    ("CSCO",  100_000.0), ("PEP",   100_000.0), ("MCD",   100_000.0),
    ("INTC",  100_000.0), ("ORCL",  100_000.0), ("IBM",   100_000.0),
    ("PLTR",  100_000.0), ("ABBV",  100_000.0), ("NFLX",  100_000.0),
    ("MRK",   100_000.0), ("PFE",   100_000.0), ("T",     100_000.0),
    ("UNH",   100_000.0), ("COST",  100_000.0), ("ABT",   100_000.0),
    ("NKE",   100_000.0), ("MDT",   100_000.0), ("LLY",   100_000.0),
    ("CRM",   100_000.0), ("LIN",   100_000.0), ("ACN",   100_000.0),
    ("LOW",   100_000.0), ("PM",    100_000.0), ("DHR",   100_000.0),
    ("TMO",   100_000.0), ("NEE",   100_000.0), ("HON",   100_000.0),
    ("TXN",   100_000.0), ("BMY",   100_000.0), ("C",     100_000.0),
    ("UPS",   100_000.0), ("BA",    100_000.0), ("GE",    100_000.0),
    ("SBUX",  100_000.0), ("AMAT",  100_000.0), ("UNP",   100_000.0),
    ("SPGI",  100_000.0), ("MS",    100_000.0), ("BLK",   100_000.0),
    ("QCOM",  100_000.0), ("GS",    100_000.0), ("CAT",   100_000.0),
    ("SCHW",  100_000.0), ("FDX",   100_000.0), ("MMM",   100_000.0),
    ("MDLZ",  100_000.0), ("GILD",  100_000.0), ("LMT",   100_000.0),
    ("DE",    100_000.0), ("BKNG",  100_000.0), ("NOW",   100_000.0),
    ("ISRG",  100_000.0), ("ADP",   100_000.0), ("EL",    100_000.0),
    ("CB",    100_000.0), ("SYK",   100_000.0), ("MO",    100_000.0),
    ("CI",    100_000.0), ("ZTS",   100_000.0), ("VRTX",  100_000.0),
    ("GM",    100_000.0), ("CL",    100_000.0), ("SO",    100_000.0),
    ("DUK",   100_000.0), ("REGN",  100_000.0), ("AON",   100_000.0),
    ("USB",   100_000.0), ("PGR",   100_000.0), ("TGT",   100_000.0),
    ("AXP",   100_000.0), ("FIS",   100_000.0),
];

/// Portfolio optimisation method to apply after computing VaR.
#[derive(Clone, ValueEnum)]
enum OptMethod {
    /// Global Minimum Variance — closed-form, short selling allowed.
    Gmv,
    /// Long-only GMV via Projected Gradient Descent (x ≥ 0).
    Pgd,
}

#[derive(Parser)]
#[command(about = "Parametric VaR calculator with optional portfolio optimisation")]
struct Args {
    /// Portfolio optimisation method (omit for a plain VaR report).
    #[arg(long, value_name = "METHOD")]
    opt: Option<OptMethod>,

    /// VaR horizon in trading days.
    #[arg(long, default_value_t = 5.0)]
    days: f64,

    /// One-tailed confidence level (e.g. 0.99 for 99%).
    #[arg(long, default_value_t = 0.99)]
    confidence_level: f64,
}

use statrs::distribution::{ContinuousCDF, Normal};

fn calculate_alpha(confidence_level: f64) -> f64 {
    let n = Normal::new(0.0, 1.0).expect("Invalid distribution parameters");
    n.inverse_cdf(confidence_level)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let alpha = calculate_alpha(args.confidence_level);
    let days = args.days;

    let portfolio: Vec<Position> = POSITIONS
        .iter()
        .map(|&(ticker, investment)| Position { ticker, investment })
        .collect();

    let all_returns: HashMap<&str, Vec<f64>> = portfolio
        .iter()
        .map(|pos| -> io::Result<_> {
            let prices = load_prices(&format!("./data/1y_{}_{}.csv", pos.ticker, today_ymd()))?;
            return Ok((pos.ticker, log_returns(&prices)));
        })
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .collect();

    let first_len = all_returns.values().next().map_or(0, |v| v.len());
    if all_returns.values().any(|v| v.len() != first_len) {
        return Err("return vectors have mismatched lengths".into());
    }

    let stats: Vec<(f64, f64)> = portfolio
        .iter()
        .map(|pos| mean_and_variance(&all_returns[pos.ticker]))
        .collect();

    let (means, variances): (Vec<f64>, Vec<f64>) = stats.into_iter().unzip();

    let cov_matrix = build_cov_matrix(&portfolio, &all_returns, &means);
    let total_wealth: f64 = portfolio.iter().map(|p| p.investment).sum();

    // ── Choose optimisation method ────────────────────────────────────────────
    let opt_weights_and_label: Option<(Vec<f64>, &str)> = match args.opt {
        Some(OptMethod::Gmv) => Some((
            gmv_portfolio(&cov_matrix, portfolio.len(), total_wealth),
            "Post-Opt (GMV)",
        )),
        Some(OptMethod::Pgd) => Some((
            pgd_portfolio(&cov_matrix, portfolio.len(), total_wealth),
            "Post-Opt (PGD)",
        )),
        None => None,
    };

    let portfolio_var =
        alpha * portfolio_dollar_variance(&portfolio, &cov_matrix).sqrt() * days.sqrt();

    // ── Per-position table ────────────────────────────────────────────────────
    if let Some((ref opt_weights, method_label)) = opt_weights_and_label {
        println!(
            "{:<8}  {:>15}  {:>12}  {:>15}  {:>12}  {:>15}",
            "Ticker", "Investment", "VaR", method_label, "Post-Opt VaR", "Invest Change",
        );
        println!(
            "{:<8}  {:>15}  {:>12}  {:>15}  {:>12}  {:>15}",
            "------",
            "---------------",
            "-----------",
            "------------------",
            "------------",
            "-------------",
        );

        let mut undiversified_var = 0.0_f64;
        let mut opt_undiversified_var = 0.0_f64;
        for (i, pos) in portfolio.iter().enumerate() {
            let sigma = variances[i].sqrt();
            let pre_var = alpha * sigma * pos.investment * days.sqrt();
            let opt_w = opt_weights[i];
            let post_var = alpha * sigma * opt_w.abs() * days.sqrt();

            undiversified_var += pre_var;
            opt_undiversified_var += post_var;

            println!(
                "{:<8}  {:>15.2}  {:>12.2}  {:>18.2}  {:>12.2}  {:>+15.2}",
                pos.ticker,
                pos.investment,
                pre_var,
                opt_w,
                post_var,
                opt_w - pos.investment,
            );
        }

        let n = portfolio.len();
        let opt_dollar_variance: f64 = (0..n)
            .flat_map(|i| (0..n).map(move |j| (i, j)))
            .map(|(i, j)| opt_weights[i] * opt_weights[j] * cov_matrix[i * n + j])
            .sum();
        let opt_portfolio_var = alpha * opt_dollar_variance.sqrt() * days.sqrt();

        let w = 36;
        println!();
        println!("{:─<80}", "");
        println!("{:<w$}  {:>14}  {:>14}", "", "Current", method_label);
        println!("{:─<80}", "");
        println!(
            "{:<w$}  {:>14.2}  {:>14.2}",
            "Undiversified VaR (sum):", undiversified_var, opt_undiversified_var
        );
        println!(
            "{:<w$}  {:>14.2}  {:>14.2}",
            "Portfolio VaR (diversified):", portfolio_var, opt_portfolio_var
        );
        println!(
            "{:<w$}  {:>14.2}  {:>14.2}",
            "Diversification benefit ($):",
            undiversified_var - portfolio_var,
            opt_undiversified_var - opt_portfolio_var
        );
        println!("{:─<80}", "");
        println!(
            "{:<w$}  {:>14}  {:>14.2}",
            "VaR reduction vs current ($):",
            "—",
            portfolio_var - opt_portfolio_var
        );
    } else {
        // No optimisation — plain VaR report
        println!("{:<8}  {:>15}  {:>12}", "Ticker", "Investment", "VaR",);
        println!(
            "{:<8}  {:>15}  {:>12}",
            "------", "---------------", "-----------",
        );

        let mut undiversified_var = 0.0_f64;
        for (i, pos) in portfolio.iter().enumerate() {
            let sigma = variances[i].sqrt();
            let var = alpha * sigma * pos.investment * days.sqrt();
            undiversified_var += var;
            println!("{:<8}  {:>15.2}  {:>12.2}", pos.ticker, pos.investment, var,);
        }

        let w = 36;
        println!();
        println!("{:─<56}", "");
        println!("{:<w$}  {:>14}", "", "Current");
        println!("{:─<56}", "");
        println!(
            "{:<w$}  {:>14.2}",
            "Undiversified VaR (sum):", undiversified_var
        );
        println!(
            "{:<w$}  {:>14.2}",
            "Portfolio VaR (diversified):", portfolio_var
        );
        println!(
            "{:<w$}  {:>14.2}",
            "Diversification benefit ($):",
            undiversified_var - portfolio_var,
        );
        println!("{:─<56}", "");
        println!("Tip: pass --opt gmv or --opt pgd to optimise the allocation.");
    }

    return Ok(());
}
