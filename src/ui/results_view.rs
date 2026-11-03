use gpui::*;
use super::app_view::AppView;
use super::{fmt_delta, fmt_money, fmt_money_int};

pub fn render_results_view(
    view: &mut AppView,
    window: &mut Window,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let summary_opt = view.state.summary.clone();
    let is_calculating = view.state.is_calculating;
    let is_filter_focused = view.results_filter_focus.is_focused(window);

    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_4()
        .bg(rgb(0x18181b))
        .rounded_lg()
        .border_1()
        .border_color(rgb(0x27272a))
        .child(
            if let Some(ref summary) = summary_opt {
                let total_w = summary.total_wealth;
                let cur_var = summary.diversified_var;
                let opt_var = summary.opt_diversified_var;
                let opt_reduction = summary.var_reduction;
                let method_name = summary
                    .opt_method_label
                    .clone()
                    .unwrap_or_else(|| "Parametric VaR".to_string());

                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        // Header title with apply button
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xf4f4f5))
                                            .child(format!("Risk & Optimization Analysis ({})", method_name)),
                                    )
                                    .children(if is_calculating {
                                        Some(
                                            div()
                                                .px_2p5()
                                                .py_0p5()
                                                .bg(rgb(0x451a03))
                                                .border_1()
                                                .border_color(rgb(0xd97706))
                                                .rounded_full()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(0xfbbf24))
                                                .child("⚡ Recalculating..."),
                                        )
                                    } else {
                                        None
                                    }),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0xa1a1aa))
                                            .child(format!(
                                                "Confidence: {:.1}% | Horizon: {:.0} trading days | Total Wealth: {}",
                                                view.state.confidence_level * 100.0,
                                                view.state.days,
                                                fmt_money(total_w)
                                            )),
                                    ),
                            )
                            .children(if opt_var.is_some() {
                                Some(
                                    div()
                                        .id("apply-opt-allocations-btn")
                                        .px_4()
                                        .py_2()
                                        .rounded_md()
                                        .bg(rgb(0x059669))
                                        .text_color(rgb(0xffffff))
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgb(0x047857)))
                                        .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                            this.apply_optimized_weights(cx);
                                        }))
                                        .child("Apply Target Allocations to Portfolio")
                                )
                            } else {
                                None
                            }),
                    )
                    .child(
                        // 4 Metric KPI Cards
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap_3()
                            .child(kpi_card(
                                "Diversified Portfolio VaR",
                                fmt_money(cur_var),
                                if opt_var.is_some() {
                                    Some("Current Allocation Baseline".to_string())
                                } else {
                                    None
                                },
                                rgb(0x38bdf8),
                            ))
                            .child(kpi_card(
                                "Undiversified VaR",
                                fmt_money(summary.undiversified_var),
                                summary.opt_undiversified_var.map(|v| format!("Post-Opt: {}", fmt_money(v))),
                                rgb(0xfb923c),
                            ))
                            .child(kpi_card(
                                "Diversification Benefit",
                                fmt_money(summary.diversification_benefit),
                                summary.opt_diversification_benefit.map(|v| format!("Post-Opt: {}", fmt_money(v))),
                                rgb(0xa78bfa),
                            ))
                            .child(kpi_card(
                                "Reduced VaR",
                                opt_var.map(fmt_money).unwrap_or_else(|| "—".to_string()),
                                opt_reduction.map(|r| {
                                    let pct = if cur_var > 0.0 { (r / cur_var) * 100.0 } else { 0.0 };
                                    if r >= 0.0 {
                                        format!("Reduced by {} ({:.1}%)", fmt_money(r), pct)
                                    } else {
                                        format!("Increased by {} ({:.1}%)", fmt_money(-r), pct.abs())
                                    }
                                }),
                                rgb(0x4ade80),
                            )),
                    )
                    .child(
                        // Rebalancing / Comparison Table
                        {
                            let filter_term = view.results_filter.trim().to_lowercase();
                            let filtered: Vec<&crate::risk::PositionVaR> = if filter_term.is_empty() {
                                summary.positions.iter().collect()
                            } else {
                                summary
                                    .positions
                                    .iter()
                                    .filter(|p| p.ticker.to_lowercase().contains(&filter_term))
                                    .collect()
                            };

                            let total_filtered = filtered.len();
                            let page_size = view.results_page_size;
                            let total_pages = if page_size == 0 || page_size == usize::MAX {
                                1
                            } else {
                                total_filtered.div_ceil(page_size)
                            }
                            .max(1);

                            let current_page = view.results_page.min(total_pages.saturating_sub(1));
                            let start_idx = if total_filtered == 0 {
                                0
                            } else {
                                current_page * page_size.min(total_filtered)
                            };
                            let end_idx = if page_size == usize::MAX {
                                total_filtered
                            } else {
                                (start_idx + page_size).min(total_filtered)
                            };
                            let page_positions = if total_filtered == 0 {
                                &[][..]
                            } else {
                                &filtered[start_idx..end_idx]
                            };
                            let cur_filter_text = view.results_filter.clone();

                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .mt_2()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(0xe4e4e7))
                                                .child("Allocation & Risk Comparison"),
                                        )
                                        .child(
                                            // Results Filter & Pagination toolbar
                                            div()
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap_3()
                                                .child(
                                                    div()
                                                        .id("results-filter-box")
                                                        .track_focus(&view.results_filter_focus)
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_2()
                                                        .px_2p5()
                                                        .py_0p5()
                                                        .bg(rgb(0x131316))
                                                        .border_1()
                                                        .border_color(if is_filter_focused {
                                                            rgb(0x3b82f6)
                                                        } else {
                                                            rgb(0x27272a)
                                                        })
                                                        .rounded_md()
                                                        .w(px(180.0))
                                                        .cursor_text()
                                                        .on_click(cx.listener(|this, _event: &ClickEvent, window, _cx| {
                                                            window.focus(&this.results_filter_focus);
                                                        }))
                                                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                                                            let key = &event.keystroke.key;
                                                            if key == "backspace" {
                                                                this.results_filter.pop();
                                                                this.results_page = 0;
                                                                cx.notify();
                                                            } else if key == "escape" {
                                                                this.results_filter.clear();
                                                                this.results_page = 0;
                                                                cx.notify();
                                                            } else if key.len() == 1 {
                                                                let c = key.chars().next().unwrap();
                                                                if !c.is_control() {
                                                                    this.results_filter.push(c);
                                                                    this.results_page = 0;
                                                                    cx.notify();
                                                                }
                                                            }
                                                        }))
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(rgb(0x71717a))
                                                                .child("🔍"),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(if cur_filter_text.is_empty() {
                                                                    rgb(0x71717a)
                                                                } else {
                                                                    rgb(0xf4f4f5)
                                                                })
                                                                .child(if cur_filter_text.is_empty() {
                                                                    if is_filter_focused { "|".to_string() } else { "Filter by ticker...".to_string() }
                                                                } else {
                                                                    cur_filter_text.clone()
                                                                }),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_1()
                                                        .child(results_page_size_button(view, 25, "25", cx))
                                                        .child(results_page_size_button(view, 50, "50", cx))
                                                        .child(results_page_size_button(view, usize::MAX, "All", cx)),
                                                )
                                                .children(if total_pages > 1 {
                                                    Some(
                                                        div()
                                                            .flex()
                                                            .flex_row()
                                                            .items_center()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .id("results-prev-page-btn")
                                                                    .px_2()
                                                                    .py_0p5()
                                                                    .rounded_sm()
                                                                    .bg(if current_page > 0 { rgb(0x27272a) } else { rgb(0x18181b) })
                                                                    .text_color(if current_page > 0 { rgb(0xf4f4f5) } else { rgb(0x52525b) })
                                                                    .text_xs()
                                                                    .cursor_pointer()
                                                                    .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                                                        if this.results_page > 0 {
                                                                            this.results_page -= 1;
                                                                            cx.notify();
                                                                        }
                                                                    }))
                                                                    .child("◀ Prev"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(rgb(0xa1a1aa))
                                                                    .child(format!("Page {}/{}", current_page + 1, total_pages)),
                                                            )
                                                            .child(
                                                                div()
                                                                    .id("results-next-page-btn")
                                                                    .px_2()
                                                                    .py_0p5()
                                                                    .rounded_sm()
                                                                    .bg(if current_page + 1 < total_pages { rgb(0x27272a) } else { rgb(0x18181b) })
                                                                    .text_color(if current_page + 1 < total_pages { rgb(0xf4f4f5) } else { rgb(0x52525b) })
                                                                    .text_xs()
                                                                    .cursor_pointer()
                                                                    .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                                                        if this.results_page + 1 < total_pages {
                                                                            this.results_page += 1;
                                                                            cx.notify();
                                                                        }
                                                                    }))
                                                                    .child("Next ▶"),
                                                            ),
                                                    )
                                                } else {
                                                    None
                                                }),
                                        ),
                                )
                                .child(
                                    // Table Header
                                    div()
                                        .flex()
                                        .flex_row()
                                        .px_3()
                                        .py_2()
                                        .bg(rgb(0x27272a))
                                        .rounded_md()
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xa1a1aa))
                                        .child(div().w(px(70.0)).child("Ticker"))
                                        .child(div().w(px(140.0)).text_right().child("Current ($)"))
                                        .child(div().w(px(130.0)).text_right().child("Pre VaR ($)"))
                                        .child(div().w(px(140.0)).text_right().child("Post-Opt ($)"))
                                        .child(div().w(px(130.0)).text_right().child("Post VaR ($)"))
                                        .child(div().flex_1().text_right().child("Rebalance Change ($)"))
                                        .child(div().w(px(100.0)).text_right().child("Weight Δ")),
                                )
                                .children(page_positions.iter().enumerate().map(|(idx, pos)| {
                                    let is_even = (start_idx + idx) % 2 == 0;
                                    let change = pos.invest_change.unwrap_or(0.0);
                                    let opt_inv = pos.opt_investment;
                                    let post_var = pos.post_var;
                                    let pct_change = if total_w > 0.0 {
                                        (change / total_w) * 100.0
                                    } else {
                                        0.0
                                    };

                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .px_3()
                                        .py_2()
                                        .rounded_md()
                                        .bg(if is_even { rgb(0x1a1a1f) } else { rgb(0x141418) })
                                        .child(
                                            div()
                                                .w(px(70.0))
                                                .text_sm()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(0x60a5fa))
                                                .child(pos.ticker.clone()),
                                        )
                                        .child(
                                            div()
                                                .w(px(140.0))
                                                .text_right()
                                                .text_xs()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(0xf4f4f5))
                                                .child(fmt_money_int(pos.investment)),
                                        )
                                        .child(
                                            div()
                                                .w(px(130.0))
                                                .text_right()
                                                .text_xs()
                                                .text_color(rgb(0xa1a1aa))
                                                .child(fmt_money(pos.single_var)),
                                        )
                                        .child(
                                            div()
                                                .w(px(140.0))
                                                .text_right()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(if opt_inv.is_some() { rgb(0x86efac) } else { rgb(0x71717a) })
                                                .child(opt_inv.map(fmt_money_int).unwrap_or_else(|| "—".to_string())),
                                        )
                                        .child(
                                            div()
                                                .w(px(130.0))
                                                .text_right()
                                                .text_xs()
                                                .text_color(rgb(0xa1a1aa))
                                                .child(post_var.map(fmt_money).unwrap_or_else(|| "—".to_string())),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_right()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(if change > 0.0 {
                                                    rgb(0x4ade80)
                                                } else if change < 0.0 {
                                                    rgb(0xf87171)
                                                } else {
                                                    rgb(0x71717a)
                                                })
                                                .child(if pos.opt_investment.is_some() {
                                                    fmt_delta(change)
                                                } else {
                                                    "—".to_string()
                                                }),
                                        )
                                        .child(
                                            div()
                                                .w(px(100.0))
                                                .text_right()
                                                .text_xs()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(if change > 0.0 {
                                                    rgb(0x4ade80)
                                                } else if change < 0.0 {
                                                    rgb(0xf87171)
                                                } else {
                                                    rgb(0x71717a)
                                                })
                                                .child(if pos.opt_investment.is_some() {
                                                    format!("{:+0.1}%", pct_change)
                                                } else {
                                                    "—".to_string()
                                                }),
                                        )
                                }))
                                .child(
                                    // Pagination row count footer
                                    if total_filtered > 0 {
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .justify_between()
                                            .pt_2()
                                            .text_xs()
                                            .text_color(rgb(0x71717a))
                                            .child(format!(
                                                "Showing {}-{} of {} asset{}",
                                                start_idx + 1,
                                                end_idx,
                                                total_filtered,
                                                if total_filtered == 1 { "" } else { "s" }
                                            ))
                                            .child(if total_pages > 1 {
                                                format!("Page {} of {}", current_page + 1, total_pages)
                                            } else {
                                                String::new()
                                            })
                                    } else {
                                        div()
                                    },
                                )
                        },
                    )
            } else {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .py_12()
                    .gap_2()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0x71717a))
                            .child("No analysis computed yet"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x52525b))
                            .child("Add at least 2 tickers to your portfolio and click 'Calculate & Optimize Portfolio'."),
                    )
            },
        )
}

fn kpi_card(
    title: &'static str,
    main_val: String,
    sub_val: Option<String>,
    accent_color: Rgba,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_48()
        .p_3()
        .bg(rgb(0x131316))
        .border_1()
        .border_color(rgb(0x27272a))
        .rounded_md()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(0xa1a1aa))
                .child(title),
        )
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(accent_color)
                .child(main_val),
        )
        .child(
            if let Some(sub) = sub_val {
                div()
                    .text_xs()
                    .text_color(rgb(0x71717a))
                    .child(sub)
            } else {
                div()
            },
        )
}

fn results_page_size_button(
    view: &AppView,
    size: usize,
    label: &'static str,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let is_selected = view.results_page_size == size;
    div()
        .id(ElementId::Name(format!("results-pagesize-{}", label).into()))
        .px_2()
        .py_0p5()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .bg(if is_selected {
            rgb(0x3b82f6)
        } else {
            rgb(0x27272a)
        })
        .text_color(if is_selected {
            rgb(0xffffff)
        } else {
            rgb(0xa1a1aa)
        })
        .hover(|s| {
            if is_selected {
                s.bg(rgb(0x2563eb))
            } else {
                s.bg(rgb(0x3f3f46))
            }
        })
        .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
            this.results_page_size = size;
            this.results_page = 0;
            cx.notify();
        }))
        .child(label)
}
