use nalgebra::{DMatrix, DVector};

/// Portfolio optimisation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OptMethod {
    /// Global Minimum Variance — closed-form analytical, short selling allowed.
    Gmv,
    /// Long-only GMV via Projected Gradient Descent (x >= 0, sum(x) = W).
    Pgd,
}

/// Global Minimum Variance (GMV) portfolio via analytical closed-form solution:
/// x* = (Σ⁻¹ 1 / 1' Σ⁻¹ 1) · W
pub fn gmv_portfolio(
    sigma_flat: &[f64],
    n: usize,
    total_wealth: f64,
) -> Result<Vec<f64>, String> {
    if n == 0 {
        return Ok(Vec::new());
    }
    if n == 1 {
        return Ok(vec![total_wealth]);
    }

    let sigma = DMatrix::from_row_slice(n, n, sigma_flat);
    let sigma_inv = match sigma.try_inverse() {
        Some(inv) => inv,
        None => {
            return Err("Covariance matrix is singular; cannot compute GMV portfolio".to_string());
        }
    };

    let ones = DVector::from_element(n, 1.0_f64);
    let sigma_inv_ones = &sigma_inv * &ones;
    let denom = ones.dot(&sigma_inv_ones);

    if denom.abs() < 1e-12 {
        return Err("Degenerate covariance denominator in GMV solution".to_string());
    }

    let x_star = sigma_inv_ones * (total_wealth / denom);
    return Ok(x_star.data.as_vec().clone());
}

/// Projects `v` in-place onto the scaled probability simplex
/// { x : x_i >= 0, Σ x_i = budget } in O(n log n).
/// Duchi et al. (2008).
pub fn project_simplex(v: &mut [f64], budget: f64) {
    if v.is_empty() {
        return;
    }
    if v.len() == 1 {
        v[0] = budget.max(0.0);
        return;
    }

    let mut u: Vec<f64> = v.to_vec();
    u.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let mut cssv: f64 = 0.0;
    let mut rho: usize = 0;

    for (j, &uj) in u.iter().enumerate() {
        cssv += uj;
        if uj - (cssv - budget) / (j + 1) as f64 > 0.0 {
            rho = j;
        }
    }

    let theta: f64 = (u[..=rho].iter().sum::<f64>() - budget) / (rho + 1) as f64;

    for xi in v.iter_mut() {
        *xi = (*xi - theta).max(0.0);
    }
}

/// Long-only Global Minimum Variance portfolio via Projected Gradient Descent.
/// Solves: min x' Σ x  s.t. x >= 0, Σ x_i = W
pub fn pgd_portfolio(sigma: &[f64], n: usize, budget: f64) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![budget.max(0.0)];
    }

    // ‖Σ‖_∞ = max absolute row sum
    let l_inf: f64 = (0..n)
        .map(|i| (0..n).map(|j| sigma[i * n + j].abs()).sum::<f64>())
        .fold(f64::NEG_INFINITY, f64::max);

    let lr = if l_inf > 1e-12 {
        1.0 / (2.0 * l_inf)
    } else {
        0.01
    };

    let mut x = vec![budget / n as f64; n];
    let mut x_new = vec![0.0_f64; n];
    let mut grad = vec![0.0_f64; n];

    let max_iter = 3000;
    for _ in 0..max_iter {
        for (i, gi) in grad.iter_mut().enumerate() {
            let row_offset = i * n;
            let mut g = 0.0_f64;
            for j in 0..n {
                g += sigma[row_offset + j] * x[j];
            }
            *gi = g;
        }

        for i in 0..n {
            x_new[i] = x[i] - lr * 2.0 * grad[i];
        }

        project_simplex(&mut x_new, budget);

        let delta: f64 = x_new.iter().zip(&x).map(|(a, b)| (a - b).abs()).sum();
        x.copy_from_slice(&x_new);

        if budget > 0.0 && delta / budget < 1e-8 {
            break;
        }
    }

    return x;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_simplex() {
        let mut v = vec![1.0, 2.0, 3.0];
        project_simplex(&mut v, 100.0);
        let sum: f64 = v.iter().sum();
        assert!((sum - 100.0).abs() < 1e-6);
        for &x in &v {
            assert!(x >= 0.0);
        }
    }

    #[test]
    fn test_pgd_two_uncorrelated_assets() {
        // Asset 1: var 0.04, Asset 2: var 0.01
        // Minimum variance portfolio should allocate more to lower variance asset
        let sigma = vec![
            0.04, 0.0,
            0.0, 0.01,
        ];
        let weights = pgd_portfolio(&sigma, 2, 1000.0);
        assert!((weights[0] + weights[1] - 1000.0).abs() < 1e-4);
        assert!(weights[1] > weights[0]);
    }
}
