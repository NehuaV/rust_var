# Portfolio VaR Risk Calculator & Optimizer

A high-performance, native desktop GUI application built with [GPUI](https://gpui.rs/) and Rust for equity portfolio risk modeling, parametric Value-at-Risk (VaR) computation, and mathematical allocation optimization (GMV and Long-Only Simplex PGD).

The application couples a parallelized Rust analytical core with an asynchronous Python bridge (`yfinance` via `uv`) to query, download, cache, and optimize portfolios ranging from small baskets to 100+ stock holdings at 60/120 FPS.

---

## Highlights & Features

- **Native GPUI Desktop GUI**: Hardware-accelerated (Metal on macOS) desktop interface designed with modern dark styling and instant sub-millisecond interaction.
- **Asynchronous Non-Blocking Engine**: All intensive computation (return vector alignment, Rayon-parallel covariance matrix construction, and gradient descent optimization) runs on background worker threads (`cx.background_spawn`), keeping the UI thread completely fluid.
- **Multi-Ticker Search & Bulk Paste**:
  - Live autocomplete backed by an internal database of US equities and ETFs.
  - Multi-ticker query bar supporting comma, semicolon, tab, or newline-delimited lists (e.g. `AAPL, MSFT, NVDA, TSLA` or index presets like `DOW`, `NASDAQ`).
  - Clipboard paste support with automatic symbol normalization (`BRK.B` $\leftrightarrow$ `BRK-B`).
- **Intelligent Local CSV Caching**: Historical daily price data is cached in `./data/` (`{period}_{TICKER}_{date}.csv`) with SHA-256 hash checks (`data/hashes.json`) to prevent redundant network fetches.
- **Dynamic Portfolio Management**:
  - Quick adjustment buttons (`-$10k`, `+$10k`, `+$50k`) and inline custom dollar amount editing per asset.
  - **Interactive Total Wealth Rescaling**: Edit aggregate portfolio wealth or apply quick multipliers (`$100k`, `$500k`, `$1M`, `$5M`, `$10M`, `2x`, `0.5x`), automatically scaling all position weights proportionally.
  - **Equal Weight Reset (`⚖️ Equal Weight`)**: Rebalance all assets to identical dollar amounts in one click while preserving total portfolio wealth.
  - **Safe Delete All (`🗑️ Delete All`)**: Two-step confirmation preventing accidental portfolio deletion.
  - **Holdings Search & Virtual Pagination**: Instant search filtering (`🔍 Filter holdings...`) and pagination (`[25]`, `[50]`, `[100]`, `[All]`) optimized for 100+ asset portfolios.
- **Optimization Algorithms**:
  - **Analytical GMV (Global Minimum Variance)**: Closed-form matrix inversion allowing unconstrained weights (short selling permitted).
  - **PGD (Projected Gradient Descent)**: Constrained quadratic programming for long-only portfolios ($x_i \ge 0, \sum x_i = W$) using Gershgorin Lipschitz step sizes and Duchi et al. (2008) $O(N \log N)$ simplex projection with zero-allocation inner loops.
  - **None (Plain VaR)**: Baseline risk report on current allocation.
- **Comprehensive Risk Comparison**:
  - High-level metric KPI cards: Diversified Portfolio VaR, Undiversified VaR, Diversification Benefit, and Reduced VaR (post-opt) with dollar and percentage reduction.
  - Full allocation rebalancing comparison table (pre- vs. post-opt dollar amounts, single-asset VaR, and percentage weight changes).
  - One-click **"Apply Target Allocations to Portfolio"** rebalancing.

---

## Quick Start

### 1. Prerequisites
- **Rust Toolchain**: Stable Rust 1.80+ (recommended: `rustup update stable`).
- **Python & uv**: Fast Python package manager [uv](https://github.com/astral-sh/uv).
- **Operating System**: macOS (Apple Silicon or Intel) with Metal graphics support.

### 2. Setup & Installation

Clone the repository and install Python dependencies:

```bash
# Clone repository
git clone https://github.com/your-username/rust_var.git
cd rust_var

# Sync python virtual environment with uv
uv sync
```

### 3. Launch Desktop Application

Run the native desktop GUI:

```bash
# Launch GUI in development mode
cargo run

# Launch in Release mode
cargo run --release

# Or build the release binary
cargo build --release
./target/release/rust_var
```

---

## Application Walkthrough

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 📊 VaR Risk Calculator & Optimizer      [📁 Portfolio Holdings] [📊 Results] │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🟢 Cache: ./data/ (daily) • Ready. Add tickers to begin risk analysis.      │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🔍 Search: AAPL, MSFT, NVDA, GOOGL ...        [📋 Paste] [＋ Add Tickers]   │
├─────────────────────────────────────────────────────────────────────────────┤
│ Portfolio Holdings (97 assets)  [⚖️ Equal] [🗑️ Delete] │ Total: $1,000,000.00│
│ 🔍 Filter holdings: [        ]           Show: [25] [50] [All]  [◀ Page 1/4 ▶] │
├───────┬──────────────────────┬──────────────┬─────────────┬─────────────────┤
│ AAPL  │ Apple Inc.           │  $10,309.28  │   $572.10   │ -$10k +$10k Edit│
│ MSFT  │ Microsoft Corp.      │  $10,309.28  │   $514.80   │ -$10k +$10k Edit│
│ NVDA  │ NVIDIA Corp.         │  $10,309.28  │   $981.40   │ -$10k +$10k Edit│
└───────┴──────────────────────┴──────────────┴─────────────┴─────────────────┘
```

### 📁 Portfolio Holdings Tab
1. **Search & Add**:
   - Type individual tickers or paste comma-separated lists (`AAPL, MSFT, AMZN, META, TSLA`).
   - Autocomplete dropdown offers company names and symbol confirmation.
   - Click **`＋ Add Tickers`** or press <kbd>Enter</kbd>. Cached files are ingested instantly; missing files are downloaded in the background via `yfinance`.
2. **Managing Investments**:
   - Use the **Quick Adjust** buttons (`-$10k`, `+$10k`, `+$50k`) or click **`Edit`** to type an exact position size.
   - Click the green **`Total Invested Wealth`** banner or **`✏️ Edit Total`** button to open the total wealth scaling dialog. Enter a new total (e.g., `$2,000,000`) or select a preset (`$500k`, `$1M`, `$5M`, `2x`, `0.5x`) to rescale all positions proportionally.
   - Click **`⚖️ Equal Weight`** to equalize all positions ($\frac{W}{N}$ per stock).
   - Click **`🗑️ Delete All`** for safe two-step clearing of the portfolio.

### 📊 Optimization & Results Tab
1. **Model Configuration**:
   - **Confidence Level**: Toggle between 90%, 95%, 99%, or 99.9% (computes exact normal quantiles via $\Phi^{-1}$).
   - **Horizon**: Select trading day horizons (1d, 5d, 10d, 21d).
   - **Method**: Select `PGD (Long-Only)`, `GMV (Unconstrained)`, or `None (Plain VaR)`.
   - Click **`⚡ Calculate & Optimize Portfolio`** (or <kbd>Enter</kbd>).
2. **KPI Summary Cards**:
   - **Diversified Portfolio VaR**: Pre-optimization baseline risk under empirical covariance.
   - **Undiversified VaR**: Worst-case risk assuming perfect cross-asset correlation ($\sum \text{VaR}_i$).
   - **Diversification Benefit**: Capital risk reduction achieved through portfolio correlation diversification.
   - **Reduced VaR**: Post-optimization portfolio VaR amount, with dollar and percentage reduction displayed on the bottom small text.
3. **Rebalancing Table**:
   - Displays each asset's current vs. target dollar allocation, pre- vs. post-VaR, rebalance change ($\pm\$$), and portfolio weight delta ($\pm\%$).
   - Click **`Apply Target Allocations to Portfolio`** to execute the target weights back to the active holdings table.

---

## Standalone Python Downloader CLI

The included `data_downloader.py` script can also be executed independently via `uv`:

```bash
# Download default sample tickers (AAPL, MSFT, NVDA, etc.)
uv run data_downloader.py

# Ensure a specific ticker is cached (outputs structured JSON)
uv run data_downloader.py ensure AAPL --period 1y

# Query ticker validity and metadata
uv run data_downloader.py search MSFT

# Batch download entire major index baskets
uv run data_downloader.py index sp500
uv run data_downloader.py index dow
uv run data_downloader.py index nasdaq
```

---

## Mathematical Foundations

### 1. Log Returns
Daily prices $P_t$ are converted to continuously compounded returns:

$$r_t = \ln\left(\frac{P_t}{P_{t-1}}\right)$$

Log returns ensure statistical additivity across multi-day horizons.

### 2. Welford's Numerically Stable Mean & Variance
Sample mean $\bar{r}$ and sum of squared deviations $M_2$ are computed in a single pass to eliminate catastrophic floating-point cancellation:

$$\delta_t = r_t - \bar{r}_{t-1}, \qquad \bar{r}_t = \bar{r}_{t-1} + \frac{\delta_t}{t}, \qquad M_{2,t} = M_{2,t-1} + (r_t - \bar{r}_t)\,\delta_t$$

$$\sigma^2 = \frac{M_{2,T}}{T - 1}$$

### 3. Rayon-Parallel Symmetrical Covariance Matrix
Sample covariance between asset $i$ and $j$ over $T$ common trading days:

$$\Sigma_{ij} = \frac{1}{T-1} \sum_{t=1}^{T} (r_{i,t} - \bar{r}_i)(r_{j,t} - \bar{r}_j)$$

The $N \times N$ matrix $\mathbf{\Sigma}$ is computed across all available CPU cores using Rayon parallel chunks (`sigma.par_chunks_mut(n)`), calculating the upper triangle in $<1\text{ ms}$ for 100+ assets.

### 4. Parametric Value-at-Risk (VaR)
Given confidence level $(1 - p)$, horizon $T$, and normal quantile $\alpha = \Phi^{-1}(1 - p)$:

- **Single Asset VaR**:
  $$\text{VaR}_i = \alpha \cdot \sigma_i \cdot x_i \cdot \sqrt{T}$$

- **Undiversified VaR** (assuming $\rho_{ij} = 1$):
  $$\text{UVaR} = \sum_{i=1}^N \text{VaR}_i$$

- **Diversified Portfolio VaR**:
  $$\text{VaR}_\Pi = \alpha \cdot \sqrt{\mathbf{x}^\top \mathbf{\Sigma} \mathbf{x}} \cdot \sqrt{T}$$

- **Diversification Benefit**:
  $$\text{Benefit} = \text{UVaR} - \text{VaR}_\Pi$$

---

## Portfolio Optimization

### A. Global Minimum Variance (GMV — Analytical)
Solves for unconstrained weights $x^*$ that minimize total variance under a fixed budget $W$:

$$\min_{\mathbf{x}} \; \mathbf{x}^\top \mathbf{\Sigma} \mathbf{x} \quad \text{subject to} \quad \mathbf{1}^\top \mathbf{x} = W$$

Formulating the Lagrangian $\mathcal{L}(\mathbf{x}, \lambda) = \mathbf{x}^\top \mathbf{\Sigma} \mathbf{x} - \lambda(\mathbf{1}^\top \mathbf{x} - W)$ yields the unique closed-form solution:

$$\mathbf{x}^* = \frac{\mathbf{\Sigma}^{-1} \mathbf{1}}{\mathbf{1}^\top \mathbf{\Sigma}^{-1} \mathbf{1}} \cdot W$$

Computed using `nalgebra::DMatrix::try_inverse()` dense LU decomposition.

### B. Long-Only GMV (PGD — Simplex Projection)
Enforces the non-negativity constraint ($x_i \ge 0$, no short selling):

$$\min_{\mathbf{x}} \; \mathbf{x}^\top \mathbf{\Sigma} \mathbf{x} \quad \text{subject to} \quad \mathbf{x} \ge \mathbf{0}, \; \mathbf{1}^\top \mathbf{x} = W$$

The iterative Projected Gradient Descent algorithm operates as:

1. **Gradient Step**:
   $$\mathbf{x}^{(k+1/2)} = \mathbf{x}^{(k)} - \eta \cdot 2\mathbf{\Sigma} \mathbf{x}^{(k)}$$
   where the optimal step size $\eta = \frac{1}{2\|\mathbf{\Sigma}\|_\infty}$ is derived from the Gershgorin circle theorem bound $\|\mathbf{\Sigma}\|_\infty = \max_i \sum_j |\Sigma_{ij}|$.

2. **Simplex Projection (Duchi et al., 2008)**:
   Projects candidate vectors $\mathbf{v}$ onto the probability simplex in $O(N \log N)$ by determining threshold $\theta$:
   $$x_i^{(k+1)} = \max(v_i - \theta, 0)$$

3. **Convergence**:
   Preallocated gradient vectors and zero heap allocations inside the loop ensure convergence in $<15\text{ ms}$ for 100 assets.

---

## Performance & Architecture

| Component | Strategy | Performance |
|-----------|----------|-------------|
| **UI Frame Rate** | Decoupled background task executor (`cx.background_spawn`) | Constant 60/120 FPS |
| **Covariance Matrix** | Rayon multi-threaded CPU parallelization (`build_cov_matrix`) | $<1\text{ ms}$ ($97 \times 97$) |
| **Simplex PGD Optimizer** | Preallocated gradient buffers, Gershgorin step size | $<15\text{ ms}$ ($97$ assets) |
| **DOM Element Count** | Virtual pagination (25/50/100/All items per view) | $\sim 300$ elements (down from 1,500+) |
| **VaR Lookup Map** | Pre-indexed $O(1)$ `HashMap<&str, f64>` per frame | $0\text{ ms}$ (eliminates 10k string comparisons) |
| **Local Cache** | SHA-256 daily file hashing in `./data/` | Instantaneous local hit |

---

## Running Unit Tests

Run the full automated test suite verifying mathematical calculations, simplex projection, date normalization, money formatting, and 97-asset stress optimization:

```bash
cargo test -- --nocapture
```

Run the linter:

```bash
cargo clippy -- -D warnings
```

---

## License

MIT License. See [LICENSE](LICENSE) for details.
