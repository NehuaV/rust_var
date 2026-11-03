use gpui::*;
use crate::python_bridge::PythonBridge;
use crate::risk::{
    compute_portfolio_var, find_cached_csv, load_prices, log_returns, OptMethod,
};
use crate::state::{AppState, Tab};
use crate::ticker_db::{lookup_name, search_tickers, TickerInfo};
use super::controls::render_controls;
use super::portfolio_table::render_portfolio_table;
use super::results_view::render_results_view;
use super::ticker_search::render_ticker_search;

/// Normalizes pasted text (converting newlines, tabs, and carriage returns to a clean comma-separated list)
pub fn normalize_pasted_text(text: &str) -> String {
    let tokens: Vec<&str> = text
        .split([',', ';', '\n', '\r', '\t'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    return tokens.join(", ").to_ascii_uppercase();
}

/// Extracts the prefix and the active search token currently being typed in a multi-ticker query.
/// Example:
/// "AAPL, MSFT, TS" -> prefix: "AAPL, MSFT, ", token: "TS"
/// "AAPL,MSFT" -> prefix: "AAPL,", token: "MSFT"
/// "TSLA" -> prefix: "", token: "TSLA"
/// "AAPL, " -> prefix: "AAPL, ", token: ""
pub fn get_current_search_token(input: &str) -> (&str, &str) {
    if let Some(idx) = input.rfind([',', ';']) {
        let prefix = &input[..=idx];
        let token = input[idx + 1..].trim();
        return (prefix, token);
    }
    return ("", input.trim());
}

pub struct AppView {
    pub state: AppState,
    pub search_focus: FocusHandle,
    pub edit_focus: FocusHandle,
    pub filter_focus: FocusHandle,
    pub results_filter_focus: FocusHandle,
    pub total_wealth_focus: FocusHandle,
    pub force_redownload: bool,
    pub editing_ticker: Option<String>,
    pub editing_buffer: String,
    pub editing_total_wealth: bool,
    pub total_wealth_buffer: String,
    pub confirming_delete_all: bool,
    pub selected_autocomplete_idx: Option<usize>,
    pub calc_epoch: u64,
    pub portfolio_page: usize,
    pub portfolio_page_size: usize,
    pub portfolio_filter: String,
    pub results_page: usize,
    pub results_page_size: usize,
    pub results_filter: String,
}

impl AppView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let search_focus = cx.focus_handle();
        let edit_focus = cx.focus_handle();
        let filter_focus = cx.focus_handle();
        let results_filter_focus = cx.focus_handle();
        let total_wealth_focus = cx.focus_handle();
        let mut view = Self {
            state: AppState::new(),
            search_focus,
            edit_focus,
            filter_focus,
            results_filter_focus,
            total_wealth_focus,
            force_redownload: false,
            editing_ticker: None,
            editing_buffer: String::new(),
            editing_total_wealth: false,
            total_wealth_buffer: String::new(),
            confirming_delete_all: false,
            selected_autocomplete_idx: None,
            calc_epoch: 0,
            portfolio_page: 0,
            portfolio_page_size: 25,
            portfolio_filter: String::new(),
            results_page: 0,
            results_page_size: 25,
            results_filter: String::new(),
        };

        view.load_cached_defaults_if_available(cx);
        return view;
    }

    /// Pre-load existing cached tickers from ./data/ if available
    pub fn load_cached_defaults_if_available(&mut self, cx: &mut Context<Self>) {
        let sample_defaults = ["AAPL", "MSFT", "NVDA", "AMZN"];
        let mut loaded_any = false;
        for ticker in sample_defaults {
            if let Some(cached_path) = find_cached_csv(ticker)
                && let Ok(prices) = load_prices(&cached_path, Some(500))
            {
                let name = lookup_name(ticker).unwrap_or(ticker).to_string();
                self.state.add_or_update_position_silent(
                    ticker.to_string(),
                    name,
                    100_000.0,
                    prices,
                );
                loaded_any = true;
            }
        }
        if loaded_any {
            self.trigger_recalculate_var(cx);
        }
    }

    /// Autocomplete suggestions for current active token being typed
    pub fn get_autocomplete_suggestions(&self) -> Vec<&'static TickerInfo> {
        let (_prefix, token) = get_current_search_token(&self.state.search_query);
        if token.is_empty() {
            return Vec::new();
        }
        return search_tickers(token, 6);
    }

    /// Apply an autocomplete suggestion to the active search token in the multi-ticker query
    pub fn apply_autocomplete_symbol(&mut self, symbol: &str, cx: &mut Context<Self>) {
        let (prefix, _token) = get_current_search_token(&self.state.search_query);
        let trimmed_prefix = prefix.trim_end();
        let new_query = if trimmed_prefix.is_empty() {
            format!("{symbol}, ")
        } else if trimmed_prefix.ends_with(',') || trimmed_prefix.ends_with(';') {
            format!("{trimmed_prefix} {symbol}, ")
        } else {
            format!("{trimmed_prefix}, {symbol}, ")
        };

        self.state.search_query = new_query;
        self.selected_autocomplete_idx = None;
        cx.notify();
    }

    /// Paste and normalize text into the search query (supporting multi-line, tab, comma-separated lists)
    pub fn paste_text_into_query(&mut self, text: &str, cx: &mut Context<Self>) {
        let upper = normalize_pasted_text(text);
        if upper.is_empty() {
            return;
        }

        let current = self.state.search_query.trim_end();
        if current.is_empty() {
            self.state.search_query = upper;
        } else if current.ends_with(',') || current.ends_with(';') {
            self.state.search_query = format!("{current} {upper}");
        } else {
            self.state.search_query = format!("{current}, {upper}");
        }

        self.selected_autocomplete_idx = None;
        cx.notify();
    }

    /// Parse comma/space/semicolon-separated tickers, check local cache FIRST,
    /// load cached tickers immediately, and asynchronously download only the missing ones.
    pub fn parse_and_add_tickers(&mut self, input: &str, force: bool, cx: &mut Context<Self>) {
        let trimmed_input = input.trim();
        let effective_input = if trimmed_input.eq_ignore_ascii_case("DOW") || trimmed_input.eq_ignore_ascii_case("DOW30") {
            "AAPL, AMGN, AXP, BA, CAT, CRM, CSCO, CVX, DIS, DOW, GS, HD, HON, IBM, INTC, JNJ, JPM, KO, MCD, MMM, MRK, MSFT, NKE, NVDA, PG, SHW, TRV, UNH, V, WMT"
        } else if trimmed_input.eq_ignore_ascii_case("NASDAQ") || trimmed_input.eq_ignore_ascii_case("QQQ") {
            "AAPL, MSFT, NVDA, AMZN, GOOGL, GOOG, META, TSLA, AVGO, COST, ASML, PEP, NFLX, LIN, AMD, ADBE, QCOM, TMUS, INTU, AMAT, CSCO, ISRG, TXN, HON, BKNG, CMCSA, AMGN, VRTX, ADI"
        } else {
            input
        };

        let mut raw_tickers: Vec<String> = Vec::new();
        for part in effective_input.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
            let clean = part.trim().to_uppercase().replace('.', "-");
            if !clean.is_empty() && !raw_tickers.contains(&clean) {
                raw_tickers.push(clean);
            }
        }

        if raw_tickers.is_empty() {
            return;
        }

        let mut cached_loaded = Vec::new();
        let mut missing_to_download = Vec::new();

        // 1. Check local cache FIRST for each ticker if !force
        for ticker in raw_tickers {
            if !force
                && let Some(cached_path) = find_cached_csv(&ticker)
                && let Ok(prices) = load_prices(&cached_path, Some(500))
            {
                let name = lookup_name(&ticker).unwrap_or(&ticker).to_string();
                let current_inv = self
                    .state
                    .positions
                    .iter()
                    .find(|p| p.ticker == ticker)
                    .map(|p| p.investment)
                    .unwrap_or(100_000.0);

                self.state.add_or_update_position_silent(
                    ticker.clone(),
                    name,
                    current_inv,
                    prices,
                );
                cached_loaded.push(ticker);
                continue;
            }
            missing_to_download.push(ticker);
        }

        // If all requested tickers were in local cache:
        if missing_to_download.is_empty() {
            self.state.search_query.clear();
            self.selected_autocomplete_idx = None;
            self.state.status = format!(
                "Loaded {} ticker(s) from local cache: {}",
                cached_loaded.len(),
                cached_loaded.join(", ")
            );
            self.trigger_recalculate_var(cx);
            return;
        }

        // If some or all tickers need downloading:
        self.state.is_busy = true;
        self.selected_autocomplete_idx = None;
        if !cached_loaded.is_empty() {
            self.state.status = format!(
                "Loaded {} from cache ({}). Downloading {} missing ticker(s): {}...",
                cached_loaded.len(),
                cached_loaded.join(", "),
                missing_to_download.len(),
                missing_to_download.join(", ")
            );
        } else {
            self.state.status = format!(
                "Downloading {} ticker(s) from Yahoo Finance: {}...",
                missing_to_download.len(),
                missing_to_download.join(", ")
            );
        }
        cx.notify();

        let missing_bg = missing_to_download.clone();
        let cached_loaded_names = cached_loaded.clone();

        cx.spawn(move |this: WeakEntity<AppView>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let batch_res = cx
                    .background_spawn(async move {
                        PythonBridge::batch_ensure_tickers(&missing_bg, force)
                    })
                    .await;

                this.update(&mut cx, |this: &mut AppView, cx| {
                    this.state.is_busy = false;

                    match batch_res {
                        Ok(batch) => {
                            let mut ok_added = Vec::new();
                            let mut failed_tickers = Vec::new();

                            for item in batch.results {
                                if item.success {
                                    let path_opt = item
                                        .csv_path
                                        .map(std::path::PathBuf::from)
                                        .or_else(|| find_cached_csv(&item.ticker));

                                    if let Some(path) = path_opt {
                                        match load_prices(&path, Some(500)) {
                                            Ok(prices) => {
                                                let display_name = lookup_name(&item.ticker)
                                                    .map(|s| s.to_string())
                                                    .unwrap_or(item.name);

                                                let current_inv = this
                                                    .state
                                                    .positions
                                                    .iter()
                                                    .find(|p| p.ticker == item.ticker)
                                                    .map(|p| p.investment)
                                                    .unwrap_or(100_000.0);

                                                this.state.add_or_update_position_silent(
                                                    item.ticker.clone(),
                                                    display_name,
                                                    current_inv,
                                                    prices,
                                                );
                                                ok_added.push(item.ticker);
                                            }
                                            Err(e) => {
                                                failed_tickers.push(format!("{}: {}", item.ticker, e));
                                            }
                                        }
                                    } else {
                                        failed_tickers.push(format!("{}: CSV file not found on disk", item.ticker));
                                    }
                                } else {
                                    failed_tickers.push(format!("{}: {}", item.ticker, item.message));
                                }
                            }

                            if !ok_added.is_empty() || !cached_loaded_names.is_empty() {
                                this.state.search_query.clear();
                                let mut parts = Vec::new();
                                if !cached_loaded_names.is_empty() {
                                    parts.push(format!("cached: {}", cached_loaded_names.join(", ")));
                                }
                                if !ok_added.is_empty() {
                                    parts.push(format!("downloaded: {}", ok_added.join(", ")));
                                }
                                let mut msg = format!("Added positions [{}]", parts.join(" | "));
                                if !failed_tickers.is_empty() {
                                    msg.push_str(&format!(" (Failed: {})", failed_tickers.join(", ")));
                                }
                                this.state.status = msg;
                                this.trigger_recalculate_var(cx);
                            } else if !failed_tickers.is_empty() {
                                this.state.status = format!("Download failed: {}", failed_tickers.join("; "));
                                cx.notify();
                            }
                        }
                        Err(e) => {
                            this.state.status = format!("Download error: {e}");
                            cx.notify();
                        }
                    }
                })
                .ok();
            }
        })
        .detach();
    }

    /// Cancel any in-progress download task
    pub fn cancel_current_task(&mut self, cx: &mut Context<Self>) {
        self.state.is_busy = false;
        self.state.status = "Download cancelled by user.".to_string();
        cx.notify();
    }

    /// Asynchronously recalculate VaR and run optimization on the background thread pool
    pub fn trigger_recalculate_var(&mut self, cx: &mut Context<Self>) {
        if self.state.positions.is_empty() {
            self.state.summary = None;
            self.state.is_calculating = false;
            cx.notify();
            return;
        }

        self.calc_epoch += 1;
        let epoch = self.calc_epoch;
        self.state.is_calculating = true;
        cx.notify();

        let positions_tuples: Vec<(String, f64)> = self
            .state
            .positions
            .iter()
            .map(|p| (p.ticker.clone(), p.investment))
            .collect();

        let returns_cache = self.state.returns_cache.clone();
        let days = self.state.days;
        let confidence_level = self.state.confidence_level;
        let opt_method = self.state.opt_method;
        let total_wealth = self.state.total_wealth();

        cx.spawn(move |this: WeakEntity<AppView>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let calc_result = cx
                    .background_spawn(async move {
                        let mut returns = returns_cache;
                        let mut min_len = usize::MAX;

                        for (ticker, _) in &positions_tuples {
                            if let Some(r) = returns.get(ticker) {
                                if !r.is_empty() && r.len() < min_len {
                                    min_len = r.len();
                                }
                            } else {
                                if let Some(cached_path) = find_cached_csv(ticker)
                                    && let Ok(prices) = load_prices(&cached_path, Some(500))
                                {
                                    let r = log_returns(&prices);
                                    if !r.is_empty() && r.len() < min_len {
                                        min_len = r.len();
                                    }
                                    returns.insert(ticker.clone(), r);
                                    continue;
                                }
                                return Err(format!("Missing cached prices for {ticker}"));
                            }
                        }

                        if min_len == usize::MAX || min_len < 2 {
                            return Err("Insufficient historical price points to compute VaR".to_string());
                        }

                        let mut aligned_returns: std::collections::HashMap<String, Vec<f64>> =
                            std::collections::HashMap::with_capacity(positions_tuples.len());
                        for (ticker, _) in &positions_tuples {
                            if let Some(r) = returns.get(ticker) {
                                let start_idx = r.len() - min_len;
                                aligned_returns.insert(ticker.clone(), r[start_idx..].to_vec());
                            }
                        }

                        let summary = compute_portfolio_var(
                            &positions_tuples,
                            &aligned_returns,
                            days,
                            confidence_level,
                            opt_method,
                        )?;

                        return Ok((summary, returns));
                    })
                    .await;

                this.update(&mut cx, |this: &mut AppView, cx| {
                    if this.calc_epoch != epoch {
                        return;
                    }
                    this.state.is_calculating = false;
                    match calc_result {
                        Ok((summary, updated_returns)) => {
                            this.state.returns_cache = updated_returns;
                            let pos_count = this.state.positions.len();
                            this.state.summary = Some(summary);
                            this.state.status = format!(
                                "VaR computed: Total Wealth ${:.2}, {} positions",
                                total_wealth, pos_count
                            );
                        }
                        Err(e) => {
                            this.state.summary = None;
                            this.state.status = format!("Calculation error: {e}");
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    /// Trigger recalculation / optimization
    pub fn trigger_optimize(&mut self, cx: &mut Context<Self>) {
        if self.state.positions.is_empty() {
            self.state.status = "Cannot optimize an empty portfolio".to_string();
            cx.notify();
            return;
        }

        self.state.active_tab = Tab::Results;
        self.trigger_recalculate_var(cx);
    }

    /// Update position investment amount silently and dispatch background recalculation
    pub fn update_position_investment(&mut self, ticker: &str, amount: f64, cx: &mut Context<Self>) {
        self.state.update_investment_silent(ticker, amount);
        self.trigger_recalculate_var(cx);
    }

    /// Remove position silently and dispatch background recalculation
    pub fn remove_portfolio_position(&mut self, ticker: &str, cx: &mut Context<Self>) {
        self.state.remove_position_silent(ticker);
        self.trigger_recalculate_var(cx);
    }

    pub fn set_confidence_level(&mut self, level: f64, cx: &mut Context<Self>) {
        self.state.confidence_level = level;
        self.trigger_recalculate_var(cx);
    }

    pub fn set_horizon_days(&mut self, days: f64, cx: &mut Context<Self>) {
        self.state.days = days;
        self.trigger_recalculate_var(cx);
    }

    pub fn set_opt_method(&mut self, method: Option<OptMethod>, cx: &mut Context<Self>) {
        self.state.opt_method = method;
        self.trigger_recalculate_var(cx);
    }

    /// Open total portfolio wealth editor
    pub fn start_editing_total_wealth(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cur = self.state.total_wealth();
        self.editing_total_wealth = true;
        self.total_wealth_buffer = format!("{cur:.0}");
        self.editing_ticker = None;
        window.focus(&self.total_wealth_focus);
        cx.notify();
    }

    /// Cancel total portfolio wealth editing
    pub fn cancel_editing_total_wealth(&mut self, cx: &mut Context<Self>) {
        self.editing_total_wealth = false;
        self.total_wealth_buffer.clear();
        cx.notify();
    }

    /// Confirm and apply new total portfolio wealth, rescaling positions proportionally
    pub fn confirm_edit_total_wealth(&mut self, cx: &mut Context<Self>) {
        if !self.editing_total_wealth {
            return;
        }
        self.editing_total_wealth = false;
        let clean = self.total_wealth_buffer.replace(['$', ',', ' '], "");
        if let Ok(new_total) = clean.trim().parse::<f64>() {
            let new_total = new_total.max(0.0);
            self.scale_portfolio_to_total_wealth(new_total, cx);
        } else {
            self.state.status = format!("Invalid total wealth amount: '{}'", self.total_wealth_buffer);
            cx.notify();
        }
        self.total_wealth_buffer.clear();
    }

    /// Rescale all positions proportionally to target total wealth and trigger background calculation
    pub fn scale_portfolio_to_total_wealth(&mut self, new_total: f64, cx: &mut Context<Self>) {
        if self.state.positions.is_empty() {
            self.state.status = "No positions in portfolio to allocate wealth to".to_string();
            cx.notify();
            return;
        }

        self.state.scale_to_total_wealth_silent(new_total);
        self.state.status = format!(
            "Rescaled total portfolio wealth to {} across {} assets",
            super::fmt_money(new_total),
            self.state.positions.len()
        );
        self.trigger_recalculate_var(cx);
    }

    /// Reset all positions so every stock in the portfolio has an equal investment amount.
    pub fn reset_equal_weights(&mut self, cx: &mut Context<Self>) {
        let n = self.state.positions.len();
        if n == 0 {
            self.state.status = "No positions in portfolio to reset".to_string();
            cx.notify();
            return;
        }

        self.state.reset_equal_weights_silent();
        let total = self.state.total_wealth();
        let per_asset = total / n as f64;
        self.state.status = format!(
            "Reset all {} positions to equal amount: {} each (Total {})",
            n,
            super::fmt_money(per_asset),
            super::fmt_money(total)
        );
        self.trigger_recalculate_var(cx);
    }

    /// Delete all positions from the portfolio.
    pub fn clear_all_positions(&mut self, cx: &mut Context<Self>) {
        if self.state.positions.is_empty() {
            return;
        }
        let count = self.state.positions.len();
        self.state.clear_all_positions_silent();
        self.confirming_delete_all = false;
        self.portfolio_page = 0;
        self.results_page = 0;
        self.portfolio_filter.clear();
        self.results_filter.clear();
        self.editing_ticker = None;
        self.editing_total_wealth = false;
        self.state.is_calculating = false;
        self.state.status = format!("Deleted all {} positions from portfolio", count);
        cx.notify();
    }

    /// Confirm editing an investment amount
    pub fn confirm_edit_investment(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.editing_ticker.take() {
            if let Ok(amount) = self.editing_buffer.trim().parse::<f64>() {
                self.state.update_investment_silent(&target, amount);
                self.state.status = format!("Updated {} investment to {}", target, super::fmt_money(amount));
                self.trigger_recalculate_var(cx);
            } else {
                self.state.status = format!("Invalid dollar amount: '{}'", self.editing_buffer);
                cx.notify();
            }
        }
        self.editing_buffer.clear();
    }

    /// Apply optimized target allocations to portfolio positions
    pub fn apply_optimized_weights(&mut self, cx: &mut Context<Self>) {
        if let Some(ref summary) = self.state.summary.clone() {
            for pos in &summary.positions {
                if let Some(target_w) = pos.opt_investment {
                    self.state.update_investment_silent(&pos.ticker, target_w);
                }
            }
            self.state.status = "Applied optimized allocations to active portfolio".to_string();
            self.trigger_recalculate_var(cx);
        }
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_busy = self.state.is_busy;
        let is_calculating = self.state.is_calculating;
        let active_tab = self.state.active_tab;
        let status = if is_calculating && !is_busy {
            "⚡ Calculating VaR & optimizing portfolio in background thread...".to_string()
        } else {
            self.state.status.clone()
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x0e0e11))
            .text_color(rgb(0xf4f4f5))
            .child(
                // Top App Navigation Bar
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_6()
                    .py_3()
                    .bg(rgb(0x18181b))
                    .border_b_1()
                    .border_color(rgb(0x27272a))
                    .child(
                        // App Brand
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size_8()
                                    .rounded_lg()
                                    .bg(rgb(0x2563eb))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_lg()
                                    .child("📊"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xffffff))
                                            .child("VaR Risk Calculator & Optimizer"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0xa1a1aa))
                                            .child("GPUI Desktop • Batch Downloads • Live Autocomplete"),
                                    ),
                            ),
                    )
                    .child(
                        // Navigation Tabs
                        div()
                            .flex()
                            .flex_row()
                            .gap_1()
                            .p_1()
                            .bg(rgb(0x131316))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x27272a))
                            .child(
                                div()
                                    .id("tab-portfolio")
                                    .px_4()
                                    .py_1p5()
                                    .rounded_sm()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .bg(if active_tab == Tab::Portfolio {
                                        rgb(0x27272a)
                                    } else {
                                        rgb(0x131316)
                                    })
                                    .text_color(if active_tab == Tab::Portfolio {
                                        rgb(0xffffff)
                                    } else {
                                        rgb(0xa1a1aa)
                                    })
                                    .hover(|s| s.bg(rgb(0x3f3f46)))
                                    .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                        this.state.active_tab = Tab::Portfolio;
                                        cx.notify();
                                    }))
                                    .child("📁 Portfolio Holdings"),
                            )
                            .child(
                                div()
                                    .id("tab-results")
                                    .px_4()
                                    .py_1p5()
                                    .rounded_sm()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .bg(if active_tab == Tab::Results {
                                        rgb(0x27272a)
                                    } else {
                                        rgb(0x131316)
                                    })
                                    .text_color(if active_tab == Tab::Results {
                                        rgb(0xffffff)
                                    } else {
                                        rgb(0xa1a1aa)
                                    })
                                    .hover(|s| s.bg(rgb(0x3f3f46)))
                                    .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                        this.state.active_tab = Tab::Results;
                                        cx.notify();
                                    }))
                                    .child("📊 Optimization & Results"),
                            ),
                    ),
            )
            .child(
                // Status Ribbon with Live Progress & Cancel Button
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_6()
                    .py_2()
                    .bg(if is_busy {
                        rgb(0x1e293b)
                    } else if is_calculating {
                        rgb(0x2d1f0d)
                    } else {
                        rgb(0x141418)
                    })
                    .border_b_1()
                    .border_color(rgb(0x222228))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size_2p5()
                                    .rounded_full()
                                    .bg(if is_busy {
                                        rgb(0x38bdf8)
                                    } else if is_calculating {
                                        rgb(0xfbbf24)
                                    } else {
                                        rgb(0x22c55e)
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if is_busy {
                                        rgb(0x38bdf8)
                                    } else if is_calculating {
                                        rgb(0xfbbf24)
                                    } else {
                                        rgb(0xa1a1aa)
                                    })
                                    .child(status),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .children(if is_busy {
                                Some(
                                    div()
                                        .id("cancel-download-btn")
                                        .px_3()
                                        .py_1()
                                        .rounded_sm()
                                        .bg(rgb(0x7f1d1d))
                                        .hover(|s| s.bg(rgb(0x991b1b)))
                                        .text_color(rgb(0xfecaca))
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                            this.cancel_current_task(cx);
                                        }))
                                        .child("✕ Cancel"),
                                )
                            } else {
                                None
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x71717a))
                                    .child("Cache: ./data/ (daily) • 25s Timeout Protection"),
                            ),
                    ),
            )
            .child(
                // Main Content Body (scrollable container)
                div()
                    .id("main-scroll-view")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_6()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_6()
                            .max_w(px(1200.0))
                            .mx_auto()
                            .child(match active_tab {
                                Tab::Portfolio => div()
                                    .flex()
                                    .flex_col()
                                    .gap_4()
                                    .child(render_ticker_search(self, window, cx))
                                    .child(render_portfolio_table(self, window, cx))
                                    .child(render_controls(self, window, cx)),
                                Tab::Results => div()
                                    .flex()
                                    .flex_col()
                                    .gap_4()
                                    .child(render_results_view(self, window, cx))
                                    .child(render_controls(self, window, cx)),
                            }),
                    ),
            )
    }
}
