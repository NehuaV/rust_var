# Portfolio VaR Calculator (Rust)

A high-performance Value-at-Risk engine written in Rust. Given a portfolio of
equity positions and a directory of historical daily close-price CSV files, it
computes:

1. **Individual VaR** for each position (parametric, normal assumption).
2. **Diversified Portfolio VaR** using the full variance-covariance matrix.
3. **Optimal Allocation** (optional) — the investment vector that minimises
   portfolio variance (and therefore VaR) for the same total budget.

---

## Quick Start

```bash
# Install python environment
uv sync

# Download data from yfinance
uv run data_downloader.py

# Build (release profile required for performance)
CARGO_TARGET_DIR=./target cargo build --release

# Run — default (plain VaR report, 99% confidence, 5-day horizon)
./target/release/rust_var

# Run — GMV optimisation, defaults
./target/release/rust_var --opt gmv

# Run — long-only PGD optimisation, defaults
./target/release/rust_var --opt pgd

# Run — custom horizon and confidence level
./target/release/rust_var --opt gmv --days 10 --confidence-level 0.95

# Show all options
./target/release/rust_var --help
```

> **Note:**  
> Make sure that all tickers listed in `main.rs` have corresponding CSV files downloaded using `data_downloader.py`.  
> You can quickly customize which tickers are fetched by editing the `DEFAULT_TICKERS` array in `data_downloader.py`.

Data files must be named `1y_<TICKER>_<YYYY-MM-DD>.csv` (the downloader writes
today's date automatically) and contain at least two columns `Date,Close` with
a single header row.

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--opt <METHOD>` | *(none)* | Optimisation method: `gmv` (short selling allowed) or `pgd` (long-only). Omit for a plain VaR report. |
| `--days <N>` | `5` | VaR horizon in trading days ($T$). |
| `--confidence-level <P>` | `0.99` | One-tailed confidence level; the corresponding $\alpha$ is computed via the normal inverse CDF. |

Run `./target/release/rust_var --help` to see all options with their defaults.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing (`--opt`, `--days`, `--confidence-level`) |
| `nalgebra` | Dense matrix inversion for the GMV closed-form solution |
| `statrs` | Normal inverse CDF for converting a confidence level to $\alpha$ |

---

## Mathematics

### Log Returns

Raw prices are converted to continuously compounded log returns before any
statistical work:

$$r_t = \ln\!\left(\frac{P_t}{P_{t-1}}\right)$$

Log returns are additive over time and better satisfy the normality assumption
than simple returns.

### Mean & Variance — Welford's Single-Pass Algorithm

Rather than making two passes (one for the mean, one for the variance), the
code uses **Welford's online algorithm** which accumulates both in a single
loop and is numerically stable:

$$\delta_t = r_t - \bar{r}_{t-1}, \qquad
\bar{r}_t = \bar{r}_{t-1} + \frac{\delta_t}{t}, \qquad
M_{2,t} = M_{2,t-1} + (r_t - \bar{r}_t)\,\delta_t$$

Sample variance: $s^2 = M_{2,T} / (T - 1)$

### Covariance Matrix

The $N \times N$ variance-covariance matrix $\Sigma$ is built from sample
covariances:

$$\Sigma_{ij} = \frac{1}{T-1}\sum_{t=1}^{T}(r_{i,t} - \bar{r}_i)(r_{j,t} - \bar{r}_j)$$

Because $\Sigma$ is symmetric ($\Sigma_{ij} = \Sigma_{ji}$), only the upper
triangle is computed (≈ half the work). The lower triangle is filled by
reflection. Pre-computed means are passed in so the mean is never recalculated
inside the inner loop.

### Parametric VaR (Normal)

Under the normality assumption, the $T$-day VaR at confidence level $(1-p)$
for a single position with dollar notional $x_i$ is:

$$\text{VaR}_i = \alpha\,\sigma_i\,x_i\,\sqrt{T}$$

where $\alpha = \Phi^{-1}(1-p)$ is the one-tailed normal quantile (e.g.
$\alpha \approx 2.33$ at the default confidence level $1-p = 99\%$),
$\sigma_i$ is the daily return standard deviation, and $T$ is the VaR horizon
in trading days. Both $(1-p)$ and $T$ are set via `--confidence-level` and
`--days` (defaults: `0.99` and `5`).

The **undiversified VaR** sums individual VaRs:

$$U\text{VaR}_\Pi = \sum_i \text{VaR}_i$$

This assumes perfect positive correlation across all positions — a worst-case
upper bound.

### Diversified Portfolio VaR

With dollar investments collected into the vector $\mathbf{x} \in \mathbb{R}^N$,
portfolio dollar variance is the quadratic form:

$$\sigma_\Pi^2 = \mathbf{x}^\top \Sigma\, \mathbf{x}$$

The diversified portfolio VaR over $T$ days is:

$$\text{VaR}_\Pi(1-p;\,T) = \alpha\sqrt{\mathbf{x}^\top \Sigma\, \mathbf{x}}\,\sqrt{T}$$

The **diversification benefit** is $U\text{VaR}_\Pi - \text{VaR}_\Pi$ — the
risk reduction from holding assets that do not move in perfect lockstep.

---

## GMV Optimal Portfolio

### Problem Statement

Given a fixed total budget $W = \sum_i x_i$, find the investment vector
$\mathbf{x}^*$ that minimises portfolio variance:

$$\min_{\mathbf{x}} \; \mathbf{x}^\top \Sigma\, \mathbf{x}
\quad \text{subject to} \quad \mathbf{1}^\top \mathbf{x} = W$$

Short selling is allowed (no non-negativity constraint). This is the
**Global Minimum Variance (GMV)** portfolio.

### Closed-Form Derivation

Form the Lagrangian:

$$\mathcal{L}(\mathbf{x}, \lambda) = \mathbf{x}^\top \Sigma\, \mathbf{x}
  - \lambda\!\left(\mathbf{1}^\top \mathbf{x} - W\right)$$

Setting the gradient to zero:

$$\frac{\partial \mathcal{L}}{\partial \mathbf{x}} = 2\,\Sigma\,\mathbf{x} - \lambda\,\mathbf{1} = \mathbf{0}
\implies \mathbf{x} = \frac{\lambda}{2}\,\Sigma^{-1}\mathbf{1}$$

Substitute into the budget constraint $\mathbf{1}^\top \mathbf{x} = W$:

$$\frac{\lambda}{2} = \frac{W}{\mathbf{1}^\top \Sigma^{-1} \mathbf{1}}$$

Therefore the unique optimal solution is:

$$\boxed{\mathbf{x}^* = \frac{\Sigma^{-1}\mathbf{1}}{\mathbf{1}^\top \Sigma^{-1}\mathbf{1}} \cdot W}$$

### Implementation

`nalgebra`'s `DMatrix::try_inverse()` performs LU decomposition to compute
$\Sigma^{-1}$. For a 90-asset portfolio the matrix is 90 × 90 — small enough
that dense LU is optimal.

```rust
let sigma     = DMatrix::from_row_slice(n, n, sigma_flat);
let sigma_inv = sigma.try_inverse().expect("Σ is singular");
let ones      = DVector::from_element(n, 1.0);
let s_inv_1   = &sigma_inv * &ones;            // Σ⁻¹ · 1
let denom     = ones.dot(&s_inv_1);            // 1ᵀ Σ⁻¹ 1
let x_star    = s_inv_1 * (total_wealth / denom);
```

### Interpretation of Results

| Metric | Equal-weight | `--opt gmv` | `--opt pgd` |
|--------|-------------|-------------|-------------|
| $\text{VaR}_\Pi$ (99%, 5-day) | \$298,804 | \$148,719 | \$178,486 |
| VaR reduction | — | **–\$150,085 (–50%)** | **–\$120,318 (–40%)** |

`--opt gmv` achieves the lower theoretical minimum but requires short positions
(negative allocations). `--opt pgd` gives up ~\$30k of VaR reduction in
exchange for a fully investable, long-only portfolio — no broker margin account
needed.

The PGD solution concentrates heavily in the lowest-volatility names
(`BRK-B`, `DUK`, `KO`, `JNJ`, `MCD`) and zeros out high-volatility, high-
correlation names (`INTC`, `TSLA`, `CI`, `AMAT`, etc.).

---

## Long-Only GMV Portfolio — `--opt pgd`

### Problem Statement

Add a non-negativity constraint to the GMV problem:

$$\min_{\mathbf{x}} \; \mathbf{x}^\top \Sigma\, \mathbf{x}
\quad \text{s.t.} \quad \mathbf{1}^\top \mathbf{x} = W,\quad x_i \geq 0 \;\forall i$$

The feasible set is the **scaled probability simplex** $\mathcal{S} = \{\mathbf{x} : x_i \geq 0,\; \mathbf{1}^\top\mathbf{x} = W\}$ — no closed-form solution exists; we need iterative optimisation.

### Projected Gradient Descent

Each iteration applies two steps:

**1. Gradient step** — move in the steepest descent direction:

$$\mathbf{x}^{(k+1/2)} = \mathbf{x}^{(k)} - \eta \cdot 2\Sigma\mathbf{x}^{(k)}$$

**2. Projection step** — snap back onto the constraint set:

$$\mathbf{x}^{(k+1)} = \Pi_{\mathcal{S}}\!\left(\mathbf{x}^{(k+1/2)}\right)$$

where $\mathcal{S} = \{x : x \geq 0,\; \mathbf{1}^\top x = W\}$.

### Step Size — Gershgorin Bound

The optimal (largest safe) step size is $\eta = 1/(2\lambda_{\max}(\Sigma))$.
We bound $\lambda_{\max}$ from above using the **Gershgorin circle theorem**:

$$\lambda_{\max}(\Sigma) \leq \max_i \sum_j |\Sigma_{ij}| = \|\Sigma\|_\infty$$

Therefore $\eta = 1 / (2\|\Sigma\|_\infty)$, which is guaranteed to be
a valid step size and is computed automatically at runtime.

### Simplex Projection — Duchi et al. (2008)

Projecting onto the simplex $\{x \geq 0, \sum x_i = W\}$ reduces to finding
the threshold $\theta$ that simultaneously satisfies the budget constraint and
clips all components to zero. The algorithm runs in $O(n \log n)$:

1. Sort $\mathbf{v}$ in descending order: $u_1 \geq u_2 \geq \ldots \geq u_n$.
2. Find the largest $\rho$ such that $u_\rho > \frac{1}{\rho+1}\!\left(\sum_{j \leq \rho} u_j - W\right)$.
3. Set $\theta = \frac{1}{\rho+1}\!\left(\sum_{j \leq \rho} u_j - W\right)$.
4. Return $x_i = \max(v_i - \theta,\; 0)$.

### Convergence

The algorithm stops when the total weight movement per iteration drops below
$W \times 10^{-9}$ (or after 10 000 iterations). For a 90-stock portfolio
convergence typically occurs in **< 500 iterations** because the step size is
large enough to move full position sizes per step.

```rust
// Key loop
for _ in 0..10_000 {
    grad[i] = 2 * Σ * x;          // O(N²) matrix-vector product
    x_new   = x - lr * grad;       // gradient step
    project_simplex(&mut x_new, W); // O(N log N) projection
    if Σ|x_new - x| / W < 1e-9 { break; }
    x = x_new;
}
```

- **Short selling permitted (`--opt gmv` only).** The long-only constraint
  $x_i \geq 0\;\forall i$ is enforced by `--opt pgd`.
- **Normal returns assumption.** The parametric VaR formula
  $\text{VaR}_\Pi = \alpha\sqrt{\mathbf{x}^\top\Sigma\mathbf{x}}\,\sqrt{T}$
  underestimates tail risk for assets with fat tails (e.g. `TSLA`, `INTC`).
  Historical simulation or Monte Carlo VaR would be more robust.
- **Estimation error in $\Sigma$.** With only one year of daily data (~252
  observations) the sample covariance matrix is noisy for $N = 90$ assets.
  Shrinkage estimators (Ledoit–Wolf) or factor models can improve stability.
- **Static allocation.** The model ignores transaction costs, liquidity
  constraints, and does not rebalance over time.

