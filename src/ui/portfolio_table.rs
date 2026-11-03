use gpui::*;
use std::collections::HashMap;
use super::app_view::AppView;
use super::fmt_money;

pub fn render_portfolio_table(
    view: &mut AppView,
    window: &mut Window,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let positions = view.state.positions.clone();
    let total_wealth = view.state.total_wealth();
    let summary = view.state.summary.clone();
    let editing_ticker = view.editing_ticker.clone();
    let editing_buffer = view.editing_buffer.clone();
    let is_modal_focused = view.edit_focus.is_focused(window);
    let is_filter_focused = view.filter_focus.is_focused(window);
    let is_wealth_focused = view.total_wealth_focus.is_focused(window);
    let editing_total_wealth = view.editing_total_wealth;
    let total_wealth_buffer = view.total_wealth_buffer.clone();
    let confirming_delete_all = view.confirming_delete_all;
    let is_calculating = view.state.is_calculating;

    // O(1) single-asset VaR lookup map (pre-indexed to avoid O(N^2) scans)
    let var_map: HashMap<&str, f64> = summary
        .as_ref()
        .map(|s| {
            s.positions
                .iter()
                .map(|p| (p.ticker.as_str(), p.single_var))
                .collect()
        })
        .unwrap_or_default();

    // Filter positions by query
    let filter_term = view.portfolio_filter.trim().to_lowercase();
    let filtered_positions: Vec<&crate::state::PositionItem> = if filter_term.is_empty() {
        positions.iter().collect()
    } else {
        positions
            .iter()
            .filter(|p| {
                p.ticker.to_lowercase().contains(&filter_term)
                    || p.name.to_lowercase().contains(&filter_term)
            })
            .collect()
    };

    let total_filtered = filtered_positions.len();
    let page_size = view.portfolio_page_size;
    let total_pages = if page_size == 0 || page_size == usize::MAX {
        1
    } else {
        total_filtered.div_ceil(page_size)
    }
    .max(1);

    let current_page = view.portfolio_page.min(total_pages.saturating_sub(1));
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
        &filtered_positions[start_idx..end_idx]
    };

    let cur_filter_text = view.portfolio_filter.clone();

    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .bg(rgb(0x18181b))
        .rounded_lg()
        .border_1()
        .border_color(rgb(0x27272a))
        .child(
            // Table Header Title + Wealth Banner
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
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
                                .child("Portfolio Holdings & Investment Amounts"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .bg(rgb(0x27272a))
                                .rounded_full()
                                .text_xs()
                                .text_color(rgb(0xa1a1aa))
                                .child(format!("{} assets", positions.len())),
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
                                    .child("⚡ Recalculating VaR..."),
                            )
                        } else {
                            None
                        })
                        .children(if !positions.is_empty() {
                            Some(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_1p5()
                                    .child(
                                        div()
                                            .id("reset-equal-weights-btn")
                                            .px_2p5()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(rgb(0x1e3a5f))
                                            .hover(|s| s.bg(rgb(0x2563eb)))
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(0x93c5fd))
                                            .hover(|s| s.text_color(rgb(0xffffff)))
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                                this.reset_equal_weights(cx);
                                            }))
                                            .child("⚖️ Equal Weight"),
                                    )
                                    .child(
                                        div()
                                            .id("delete-all-positions-btn")
                                            .px_2p5()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(if confirming_delete_all {
                                                rgb(0xb91c1c)
                                            } else {
                                                rgb(0x3f1818)
                                            })
                                            .hover(|s| s.bg(if confirming_delete_all {
                                                rgb(0xdc2626)
                                            } else {
                                                rgb(0x7f1d1d)
                                            }))
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if confirming_delete_all {
                                                rgb(0xffffff)
                                            } else {
                                                rgb(0xfca5a5)
                                            })
                                            .hover(|s| s.text_color(rgb(0xffffff)))
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                                if this.confirming_delete_all {
                                                    this.clear_all_positions(cx);
                                                } else {
                                                    this.confirming_delete_all = true;
                                                    cx.notify();
                                                }
                                            }))
                                            .child(if confirming_delete_all {
                                                "⚠️ Confirm Delete All?"
                                            } else {
                                                "🗑️ Delete All"
                                            }),
                                    )
                                    .children(if confirming_delete_all {
                                        Some(
                                            div()
                                                .id("cancel-delete-all-btn")
                                                .px_2()
                                                .py_0p5()
                                                .rounded_sm()
                                                .bg(rgb(0x27272a))
                                                .hover(|s| s.bg(rgb(0x3f3f46)))
                                                .text_xs()
                                                .text_color(rgb(0xa1a1aa))
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                                    this.confirming_delete_all = false;
                                                    cx.notify();
                                                }))
                                                .child("Cancel"),
                                        )
                                    } else {
                                        None
                                    }),
                            )
                        } else {
                            None
                        }),
                )
                .child(
                    // Total Invested Wealth Banner + Edit Total Button
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .id("total-wealth-banner")
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_3()
                                .py_1p5()
                                .bg(rgb(0x0f291e))
                                .border_1()
                                .border_color(rgb(0x166534))
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0x143c2c)).border_color(rgb(0x22c55e)))
                                .on_click(cx.listener(|this, _event: &ClickEvent, window, cx| {
                                    this.start_editing_total_wealth(window, cx);
                                }))
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0x86efac))
                                        .child("Total Invested Wealth:"),
                                )
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x22c55e))
                                        .child(fmt_money(total_wealth)),
                                ),
                        )
                        .child(
                            div()
                                .id("edit-total-wealth-btn")
                                .px_2p5()
                                .py_1p5()
                                .rounded_md()
                                .bg(rgb(0x1e3a5f))
                                .hover(|s| s.bg(rgb(0x2563eb)))
                                .text_color(rgb(0x93c5fd))
                                .hover(|s| s.text_color(rgb(0xffffff)))
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _event: &ClickEvent, window, cx| {
                                    this.start_editing_total_wealth(window, cx);
                                }))
                                .child("✏️ Edit Total"),
                        ),
                ),
        )
        .children(if editing_total_wealth {
            Some(
                div()
                    .id("total-wealth-edit-dialog")
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(rgb(0x13221b))
                    .border_1()
                    .border_color(rgb(0x22c55e))
                    .rounded_md()
                    .child(
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
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x86efac))
                                            .child("Edit Total Portfolio Wealth"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0xa1a1aa))
                                            .child("Adjusts all asset dollar amounts proportionally, preserving portfolio weights."),
                                    ),
                            )
                            .child(
                                div()
                                    .id("close-total-wealth-edit-btn")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x27272a))
                                    .text_xs()
                                    .text_color(rgb(0xa1a1aa))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x3f3f46)).text_color(rgb(0xffffff)))
                                    .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                        this.cancel_editing_total_wealth(cx);
                                    }))
                                    .child("✕ Close"),
                            ),
                    )
                    .child(
                        // Input box + Action buttons
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .id("total-wealth-input-box")
                                    .track_focus(&view.total_wealth_focus)
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .px_3()
                                    .py_1p5()
                                    .bg(rgb(0x0a1410))
                                    .border_1()
                                    .border_color(if is_wealth_focused { rgb(0x22c55e) } else { rgb(0x166534) })
                                    .rounded_sm()
                                    .w(px(240.0))
                                    .cursor_text()
                                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                                        let key = &event.keystroke.key;
                                        if key == "enter" {
                                            this.confirm_edit_total_wealth(cx);
                                        } else if key == "escape" {
                                            this.cancel_editing_total_wealth(cx);
                                        } else if key == "backspace" {
                                            this.total_wealth_buffer.pop();
                                            cx.notify();
                                        } else if key.len() == 1 {
                                            let c = key.chars().next().unwrap();
                                            if c.is_ascii_digit() || c == '.' || c == ',' {
                                                this.total_wealth_buffer.push(c);
                                                cx.notify();
                                            }
                                        }
                                    }))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0x86efac))
                                            .child("$"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xffffff))
                                            .child(format!("{}{}", total_wealth_buffer, if is_wealth_focused { "|" } else { "" })),
                                    ),
                            )
                            .child(
                                div()
                                    .id("apply-total-wealth-btn")
                                    .px_4()
                                    .py_1p5()
                                    .rounded_sm()
                                    .bg(rgb(0x16a34a))
                                    .hover(|s| s.bg(rgb(0x15803d)))
                                    .text_color(rgb(0xffffff))
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                        this.confirm_edit_total_wealth(cx);
                                    }))
                                    .child("Apply Total Wealth"),
                            )
                            .child(
                                div()
                                    .id("cancel-total-wealth-btn")
                                    .px_3()
                                    .py_1p5()
                                    .rounded_sm()
                                    .bg(rgb(0x27272a))
                                    .hover(|s| s.bg(rgb(0x3f3f46)))
                                    .text_color(rgb(0xd4d4d8))
                                    .text_xs()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                        this.cancel_editing_total_wealth(cx);
                                    }))
                                    .child("Cancel"),
                            ),
                    )
                    .child(
                        // Quick Presets Row
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .flex_wrap()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xa1a1aa))
                                    .child("Quick presets:"),
                            )
                            .child(wealth_preset_pill("$100k", 100_000.0, cx))
                            .child(wealth_preset_pill("$500k", 500_000.0, cx))
                            .child(wealth_preset_pill("$1M", 1_000_000.0, cx))
                            .child(wealth_preset_pill("$5M", 5_000_000.0, cx))
                            .child(wealth_preset_pill("$10M", 10_000_000.0, cx))
                            .child(wealth_factor_pill("2x (+100%)", 2.0, cx))
                            .child(wealth_factor_pill("0.5x (-50%)", 0.5, cx))
                            .child(wealth_equal_pill("Equal Weight ($1M)", 1_000_000.0, cx)),
                    ),
            )
        } else {
            None
        })
        .child(
            // Filter Bar & Pagination Toolbar
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_4()
                .px_1()
                .child(
                    // Filter input box
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .id("portfolio-filter-box")
                                .track_focus(&view.filter_focus)
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_3()
                                .py_1()
                                .bg(rgb(0x131316))
                                .border_1()
                                .border_color(if is_filter_focused {
                                    rgb(0x3b82f6)
                                } else {
                                    rgb(0x27272a)
                                })
                                .rounded_md()
                                .w(px(260.0))
                                .cursor_text()
                                .on_click(cx.listener(|this, _event: &ClickEvent, window, _cx| {
                                    window.focus(&this.filter_focus);
                                }))
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                                    let key = &event.keystroke.key;
                                    if key == "backspace" {
                                        this.portfolio_filter.pop();
                                        this.portfolio_page = 0;
                                        cx.notify();
                                    } else if key == "escape" {
                                        this.portfolio_filter.clear();
                                        this.portfolio_page = 0;
                                        cx.notify();
                                    } else if key.len() == 1 {
                                        let c = key.chars().next().unwrap();
                                        if !c.is_control() {
                                            this.portfolio_filter.push(c);
                                            this.portfolio_page = 0;
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
                                            if is_filter_focused { "|".to_string() } else { "Filter holdings (e.g. AAPL)...".to_string() }
                                        } else {
                                            cur_filter_text.clone()
                                        }),
                                ),
                        )
                        .children(if !cur_filter_text.is_empty() {
                            Some(
                                div()
                                    .id("clear-portfolio-filter-btn")
                                    .px_2()
                                    .py_0p5()
                                    .bg(rgb(0x27272a))
                                    .hover(|s| s.bg(rgb(0x3f3f46)))
                                    .rounded_sm()
                                    .text_xs()
                                    .text_color(rgb(0xa1a1aa))
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                        this.portfolio_filter.clear();
                                        this.portfolio_page = 0;
                                        cx.notify();
                                    }))
                                    .child("Clear filter"),
                            )
                        } else {
                            None
                        })
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x71717a))
                                .child(if !cur_filter_text.is_empty() {
                                    format!("({} match)", total_filtered)
                                } else {
                                    String::new()
                                }),
                        ),
                )
                .child(
                    // Pagination & Page Size selector
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .child(
                            // Page size pills: [25] [50] [100] [All]
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x71717a))
                                        .child("Show:"),
                                )
                                .child(page_size_button(view, 25, "25", cx))
                                .child(page_size_button(view, 50, "50", cx))
                                .child(page_size_button(view, 100, "100", cx))
                                .child(page_size_button(view, usize::MAX, "All", cx)),
                        )
                        .children(if total_pages > 1 {
                            Some(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_1p5()
                                    .child(
                                        div()
                                            .id("portfolio-prev-page-btn")
                                            .px_2()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(if current_page > 0 {
                                                rgb(0x27272a)
                                            } else {
                                                rgb(0x18181b)
                                            })
                                            .text_color(if current_page > 0 {
                                                rgb(0xf4f4f5)
                                            } else {
                                                rgb(0x52525b)
                                            })
                                            .text_xs()
                                            .cursor_pointer()
                                            .hover(|s| {
                                                if current_page > 0 {
                                                    s.bg(rgb(0x3f3f46))
                                                } else {
                                                    s
                                                }
                                            })
                                            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                                if this.portfolio_page > 0 {
                                                    this.portfolio_page -= 1;
                                                    cx.notify();
                                                }
                                            }))
                                            .child("◀ Prev"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(0xa1a1aa))
                                            .child(format!("Page {} of {}", current_page + 1, total_pages)),
                                    )
                                    .child(
                                        div()
                                            .id("portfolio-next-page-btn")
                                            .px_2()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(if current_page + 1 < total_pages {
                                                rgb(0x27272a)
                                            } else {
                                                rgb(0x18181b)
                                            })
                                            .text_color(if current_page + 1 < total_pages {
                                                rgb(0xf4f4f5)
                                            } else {
                                                rgb(0x52525b)
                                            })
                                            .text_xs()
                                            .cursor_pointer()
                                            .hover(|s| {
                                                if current_page + 1 < total_pages {
                                                    s.bg(rgb(0x3f3f46))
                                                } else {
                                                    s
                                                }
                                            })
                                            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                                if this.portfolio_page + 1 < total_pages {
                                                    this.portfolio_page += 1;
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
            // Table Header Columns
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
                .child(div().flex_1().child("Asset / Company"))
                .child(div().w(px(140.0)).text_right().child("Investment ($)"))
                .child(div().w(px(130.0)).text_right().child("Single VaR ($)"))
                .child(div().w(px(240.0)).text_center().child("Adjust Investment"))
                .child(div().w(px(60.0)).text_center().child("Action")),
        )
        .child(
            if positions.is_empty() {
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
                            .child("No positions in portfolio yet"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x52525b))
                            .child("Use the search bar above or paste tickers (e.g. AAPL, MSFT) to begin."),
                    )
            } else if total_filtered == 0 {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .py_8()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xa1a1aa))
                            .child(format!("No assets matching '{}'", filter_term)),
                    )
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(page_positions.iter().enumerate().map(|(idx, pos)| {
                        let ticker = pos.ticker.clone();
                        let t_for_remove = ticker.clone();
                        let t_for_add10 = ticker.clone();
                        let t_for_sub10 = ticker.clone();
                        let t_for_add50 = ticker.clone();
                        let t_for_edit = ticker.clone();
                        let is_even = (start_idx + idx) % 2 == 0;

                        // O(1) single-asset VaR from pre-indexed map
                        let single_var_str = var_map
                            .get(ticker.as_str())
                            .map(|&v| fmt_money(v))
                            .unwrap_or_else(|| "—".to_string());

                        div()
                            .id(ElementId::Name(format!("pos-row-{}", ticker).into()))
                            .flex()
                            .flex_row()
                            .items_center()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(if is_even { rgb(0x1a1a1f) } else { rgb(0x141418) })
                            .hover(|s| s.bg(rgb(0x22222a)))
                            .child(
                                div()
                                    .w(px(70.0))
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x60a5fa))
                                    .child(ticker.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .text_color(rgb(0xd4d4d8))
                                    .overflow_hidden()
                                    .child(pos.name.clone()),
                            )
                            .child(
                                div()
                                    .w(px(140.0))
                                    .text_right()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xf4f4f5))
                                    .child(fmt_money(pos.investment)),
                            )
                            .child(
                                div()
                                    .w(px(130.0))
                                    .text_right()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(0xf87171))
                                    .child(single_var_str),
                            )
                            .child(
                                // Quick Adjust Amount Buttons (silent updates with background recalculation)
                                div()
                                    .w(px(240.0))
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .id(ElementId::Name(format!("sub10-{}", ticker).into()))
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .bg(rgb(0x27272a))
                                            .text_xs()
                                            .text_color(rgb(0xd4d4d8))
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x3f3f46)))
                                            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                                let cur = this.state.positions.iter().find(|p| p.ticker == t_for_sub10).map(|p| p.investment).unwrap_or(0.0);
                                                this.update_position_investment(&t_for_sub10, (cur - 10_000.0).max(0.0), cx);
                                            }))
                                            .child("-$10k"),
                                    )
                                    .child(
                                        div()
                                            .id(ElementId::Name(format!("add10-{}", ticker).into()))
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .bg(rgb(0x27272a))
                                            .text_xs()
                                            .text_color(rgb(0xd4d4d8))
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x3f3f46)))
                                            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                                let cur = this.state.positions.iter().find(|p| p.ticker == t_for_add10).map(|p| p.investment).unwrap_or(0.0);
                                                this.update_position_investment(&t_for_add10, cur + 10_000.0, cx);
                                            }))
                                            .child("+$10k"),
                                    )
                                    .child(
                                        div()
                                            .id(ElementId::Name(format!("add50-{}", ticker).into()))
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .bg(rgb(0x27272a))
                                            .text_xs()
                                            .text_color(rgb(0xd4d4d8))
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x3f3f46)))
                                            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                                let cur = this.state.positions.iter().find(|p| p.ticker == t_for_add50).map(|p| p.investment).unwrap_or(0.0);
                                                this.update_position_investment(&t_for_add50, cur + 50_000.0, cx);
                                            }))
                                            .child("+$50k"),
                                    )
                                    .child(
                                        div()
                                            .id(ElementId::Name(format!("edit-btn-{}", ticker).into()))
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .bg(rgb(0x1e3a5f))
                                            .text_xs()
                                            .text_color(rgb(0x93c5fd))
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x2563eb)).text_color(rgb(0xffffff)))
                                            .on_click(cx.listener(move |this, _event: &ClickEvent, window, cx| {
                                                let cur = this.state.positions.iter().find(|p| p.ticker == t_for_edit).map(|p| p.investment).unwrap_or(0.0);
                                                this.editing_ticker = Some(t_for_edit.clone());
                                                this.editing_buffer = format!("{cur:.0}");
                                                window.focus(&this.edit_focus);
                                                cx.notify();
                                            }))
                                            .child("Edit"),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(60.0))
                                    .flex()
                                    .justify_center()
                                    .child(
                                        div()
                                            .id(ElementId::Name(format!("del-{}", ticker).into()))
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .bg(rgb(0x3f1818))
                                            .text_xs()
                                            .text_color(rgb(0xfca5a5))
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x7f1d1d)))
                                            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                                this.remove_portfolio_position(&t_for_remove, cx);
                                            }))
                                            .child("✕"),
                                    ),
                            )
                    }))
            },
        )
        .child(
            // Pagination Footer / Row count info
            if total_filtered > 0 {
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .pt_2()
                    .text_xs()
                    .text_color(rgb(0x71717a))
                    .child(
                        format!(
                            "Showing {}-{} of {} asset{}",
                            start_idx + 1,
                            end_idx,
                            total_filtered,
                            if total_filtered == 1 { "" } else { "s" }
                        ),
                    )
                    .child(
                        if total_pages > 1 {
                            format!("Page {} of {}", current_page + 1, total_pages)
                        } else {
                            String::new()
                        },
                    )
            } else {
                div()
            },
        )
        .child(
            // Inline Amount Edit Dialog if active
            if let Some(ref target_ticker) = editing_ticker {
                let target = target_ticker.clone();
                let buf = editing_buffer.clone();

                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .p_3()
                    .mt_2()
                    .bg(rgb(0x1e293b))
                    .border_1()
                    .border_color(rgb(0x38bdf8))
                    .rounded_md()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x38bdf8))
                                    .child(format!("Edit Investment for {}:", target)),
                            )
                            .child(
                                div()
                                    .id("edit-buffer-box")
                                    .track_focus(&view.edit_focus)
                                    .px_3()
                                    .py_1()
                                    .bg(rgb(0x0f172a))
                                    .border_1()
                                    .border_color(if is_modal_focused { rgb(0x38bdf8) } else { rgb(0x475569) })
                                    .rounded_sm()
                                    .text_sm()
                                    .text_color(rgb(0xf8fafc))
                                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                                        let key = &event.keystroke.key;
                                        if key == "enter" {
                                            this.confirm_edit_investment(cx);
                                        } else if key == "escape" {
                                            this.editing_ticker = None;
                                            cx.notify();
                                        } else if key == "backspace" {
                                            this.editing_buffer.pop();
                                            cx.notify();
                                        } else if key.len() == 1 {
                                            let c = key.chars().next().unwrap();
                                            if c.is_ascii_digit() || c == '.' {
                                                this.editing_buffer.push(c);
                                                cx.notify();
                                            }
                                        }
                                    }))
                                    .child(format!("${}{}", buf, if is_modal_focused { "|" } else { "" })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                div()
                                    .id("confirm-edit-btn")
                                    .px_3()
                                    .py_1()
                                    .rounded_sm()
                                    .bg(rgb(0x0284c7))
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xffffff))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x0369a1)))
                                    .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                        this.confirm_edit_investment(cx);
                                    }))
                                    .child("Save Amount"),
                            )
                            .child(
                                div()
                                    .id("cancel-edit-btn")
                                    .px_3()
                                    .py_1()
                                    .rounded_sm()
                                    .bg(rgb(0x334155))
                                    .text_xs()
                                    .text_color(rgb(0xcbd5e1))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x475569)))
                                    .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                        this.editing_ticker = None;
                                        cx.notify();
                                    }))
                                    .child("Cancel"),
                            ),
                    )
            } else {
                div()
            },
        )
}

fn page_size_button(
    view: &AppView,
    size: usize,
    label: &'static str,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let is_selected = view.portfolio_page_size == size;
    div()
        .id(ElementId::Name(format!("pagesize-{}", label).into()))
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
            this.portfolio_page_size = size;
            this.portfolio_page = 0;
            cx.notify();
        }))
        .child(label)
}

fn wealth_preset_pill(
    label: &'static str,
    amount: f64,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    div()
        .id(ElementId::Name(format!("wealth-preset-{}", label).into()))
        .px_2()
        .py_0p5()
        .rounded_sm()
        .bg(rgb(0x164e33))
        .border_1()
        .border_color(rgb(0x22c55e))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(0x86efac))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x15803d)).text_color(rgb(0xffffff)))
        .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
            this.scale_portfolio_to_total_wealth(amount, cx);
            this.editing_total_wealth = false;
        }))
        .child(label)
}

fn wealth_factor_pill(
    label: &'static str,
    factor: f64,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    div()
        .id(ElementId::Name(format!("wealth-factor-{}", label).into()))
        .px_2()
        .py_0p5()
        .rounded_sm()
        .bg(rgb(0x1e293b))
        .border_1()
        .border_color(rgb(0x38bdf8))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(0x7dd3fc))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x0284c7)).text_color(rgb(0xffffff)))
        .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
            let cur = this.state.total_wealth();
            this.scale_portfolio_to_total_wealth(cur * factor, cx);
            this.editing_total_wealth = false;
        }))
        .child(label)
}

fn wealth_equal_pill(
    label: &'static str,
    target_total: f64,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    div()
        .id(ElementId::Name(format!("wealth-equal-{}", label).into()))
        .px_2()
        .py_0p5()
        .rounded_sm()
        .bg(rgb(0x2e1065))
        .border_1()
        .border_color(rgb(0xa855f7))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(0xd8b4fe))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x7e22ce)).text_color(rgb(0xffffff)))
        .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
            let n = this.state.positions.len();
            if n > 0 {
                let per_asset = target_total / n as f64;
                for pos in &mut this.state.positions {
                    pos.investment = per_asset;
                    pos.investment_input = format!("{per_asset:.0}");
                }
                this.state.status = format!(
                    "Set equal weighting ({} per asset) across {} assets for {}",
                    super::fmt_money(per_asset),
                    n,
                    super::fmt_money(target_total)
                );
                this.trigger_recalculate_var(cx);
            }
            this.editing_total_wealth = false;
        }))
        .child(label)
}
