use gpui::*;
use super::app_view::AppView;

pub fn render_ticker_search(
    view: &mut AppView,
    window: &mut Window,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let is_focused = view.search_focus.is_focused(window);
    let is_busy = view.state.is_busy;
    let query = view.state.search_query.clone();
    let force = view.force_redownload;
    let suggestions = view.get_autocomplete_suggestions();
    let selected_idx = view.selected_autocomplete_idx;
    let has_suggestions = is_focused && !suggestions.is_empty();

    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .bg(rgb(0x1e1e24))
        .rounded_lg()
        .border_1()
        .border_color(rgb(0x2e2e38))
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
                                .text_color(rgb(0xe4e4e7))
                                .child("Search & Batch Add Tickers"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x71717a))
                                .child("Type or paste single/multiple tickers (e.g. AAPL, MSFT, TSLA, V) • Cmd+V supported"),
                        ),
                )
                .child(
                    div()
                        .id("force-toggle")
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                            this.force_redownload = !this.force_redownload;
                            cx.notify();
                        }))
                        .child(
                            div()
                                .size_4()
                                .rounded_sm()
                                .border_1()
                                .border_color(if force { rgb(0x3b82f6) } else { rgb(0x52525b) })
                                .bg(if force { rgb(0x3b82f6) } else { rgb(0x27272a) })
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(if force { "✓" } else { "" }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xa1a1aa))
                                .child("Force re-download (bypass cache)"),
                        ),
                ),
        )
        .child(
            // Main Input + Action Buttons Row
            div()
                .flex()
                .flex_row()
                .gap_2()
                .items_center()
                .child(
                    // Interactive Multi-Ticker Input Field with Autocomplete & Paste Support
                    div()
                        .id("search-input-box")
                        .track_focus(&view.search_focus)
                        .flex_1()
                        .px_3()
                        .py_2()
                        .bg(rgb(0x131316))
                        .rounded_md()
                        .border_1()
                        .border_color(if is_focused {
                            rgb(0x3b82f6)
                        } else {
                            rgb(0x3f3f46)
                        })
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                            window.focus(&this.search_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                            let key = &event.keystroke.key;
                            let mods = &event.keystroke.modifiers;
                            let suggestions = this.get_autocomplete_suggestions();

                            // 1. Paste Shortcut (Cmd+V on macOS or Ctrl+V on Windows/Linux)
                            if (mods.platform || mods.control) && (key == "v" || key == "V") {
                                if let Some(item) = cx.read_from_clipboard()
                                    && let Some(text) = item.text()
                                {
                                    this.paste_text_into_query(&text, cx);
                                }
                                return;
                            }

                            // 2. Clear / Select All Shortcuts (Cmd+Backspace, Cmd+A / Ctrl+A)
                            if (mods.platform && key == "backspace")
                                || ((mods.platform || mods.control) && (key == "a" || key == "A"))
                            {
                                this.state.search_query.clear();
                                this.selected_autocomplete_idx = None;
                                cx.notify();
                                return;
                            }

                            if key == "enter" {
                                if let Some(idx) = this.selected_autocomplete_idx
                                    && idx < suggestions.len()
                                {
                                    let sym = suggestions[idx].symbol;
                                    this.apply_autocomplete_symbol(sym, cx);
                                    return;
                                }
                                let q = this.state.search_query.clone();
                                this.parse_and_add_tickers(&q, this.force_redownload, cx);
                            } else if key == "tab" {
                                if !suggestions.is_empty() {
                                    let idx = this.selected_autocomplete_idx.unwrap_or(0);
                                    if idx < suggestions.len() {
                                        let sym = suggestions[idx].symbol;
                                        this.apply_autocomplete_symbol(sym, cx);
                                    }
                                }
                            } else if key == "down" {
                                if !suggestions.is_empty() {
                                    let next = match this.selected_autocomplete_idx {
                                        Some(i) => (i + 1) % suggestions.len(),
                                        None => 0,
                                    };
                                    this.selected_autocomplete_idx = Some(next);
                                    cx.notify();
                                }
                            } else if key == "up" {
                                if !suggestions.is_empty() {
                                    let prev = match this.selected_autocomplete_idx {
                                        Some(0) | None => suggestions.len() - 1,
                                        Some(i) => i - 1,
                                    };
                                    this.selected_autocomplete_idx = Some(prev);
                                    cx.notify();
                                }
                            } else if key == "backspace" {
                                this.state.search_query.pop();
                                this.selected_autocomplete_idx = None;
                                cx.notify();
                            } else if key == "escape" {
                                this.state.search_query.clear();
                                this.selected_autocomplete_idx = None;
                                cx.notify();
                            } else if key.len() == 1 && !mods.platform && !mods.control {
                                let c = key.chars().next().unwrap();
                                if c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == ',' || c == ' ' || c == ';' {
                                    this.state.search_query.push(c.to_ascii_uppercase());
                                    this.selected_autocomplete_idx = None;
                                    cx.notify();
                                }
                            }
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .w_full()
                                .child(
                                    if query.is_empty() {
                                        div()
                                            .text_sm()
                                            .text_color(rgb(0x71717a))
                                            .child("Enter tickers or paste (e.g. AAPL, MSFT, TSLA, V)...")
                                    } else {
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(0xf4f4f5))
                                            .child(format!("{query}{}", if is_focused { "|" } else { "" }))
                                    },
                                )
                                .children(if !query.is_empty() {
                                    Some(
                                        div()
                                            .id("clear-search-btn")
                                            .px_1p5()
                                            .py_0p5()
                                            .rounded_sm()
                                            .text_xs()
                                            .text_color(rgb(0x71717a))
                                            .hover(|s| s.text_color(rgb(0xf4f4f5)).bg(rgb(0x27272a)))
                                            .cursor_pointer()
                                            .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                                this.state.search_query.clear();
                                                this.selected_autocomplete_idx = None;
                                                cx.notify();
                                            }))
                                            .child("✕ Clear"),
                                    )
                                } else {
                                    None
                                }),
                        ),
                )
                .child(
                    // Dedicated Paste Button
                    div()
                        .id("paste-ticker-btn")
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(rgb(0x27272a))
                        .border_1()
                        .border_color(rgb(0x3f3f46))
                        .text_color(rgb(0xe4e4e7))
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x3f3f46)).text_color(rgb(0xffffff)))
                        .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                            if let Some(item) = cx.read_from_clipboard()
                                && let Some(text) = item.text()
                            {
                                this.paste_text_into_query(&text, cx);
                            }
                        }))
                        .child("📋 Paste"),
                )
                .child(
                    // Add Button
                    div()
                        .id("add-ticker-btn")
                        .px_4()
                        .py_2()
                        .rounded_md()
                        .bg(if is_busy { rgb(0x4b5563) } else { rgb(0x2563eb) })
                        .text_color(rgb(0xffffff))
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x1d4ed8)))
                        .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                            let q = this.state.search_query.clone();
                            this.parse_and_add_tickers(&q, this.force_redownload, cx);
                        }))
                        .child(if is_busy { "Downloading..." } else { "+ Add to Portfolio" }),
                )
                .children(if is_busy {
                    Some(
                        div()
                            .id("cancel-btn-search")
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x7f1d1d))
                            .text_color(rgb(0xfecaca))
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x991b1b)))
                            .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                                this.cancel_current_task(cx);
                            }))
                            .child("✕ Cancel"),
                    )
                } else {
                    None
                }),
        )
        .child(
            // Autocomplete Dropdown overlay if active
            if has_suggestions {
                div()
                    .flex()
                    .flex_col()
                    .bg(rgb(0x18181b))
                    .border_1()
                    .border_color(rgb(0x3b82f6))
                    .rounded_md()
                    .p_1()
                    .gap_1()
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(rgb(0x71717a))
                            .child("Autofill Suggestions (click or use ↑↓ and Enter/Tab):"),
                    )
                    .children(suggestions.iter().enumerate().map(|(idx, info)| {
                        let sym = info.symbol.to_string();
                        let is_highlighted = selected_idx == Some(idx);
                        div()
                            .id(ElementId::Name(format!("ac-{}", info.symbol).into()))
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded_sm()
                            .cursor_pointer()
                            .bg(if is_highlighted { rgb(0x27272a) } else { rgb(0x18181b) })
                            .hover(|s| s.bg(rgb(0x27272a)))
                            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                this.apply_autocomplete_symbol(&sym, cx);
                            }))
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
                                            .text_color(rgb(0x60a5fa))
                                            .child(info.symbol),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0xf4f4f5))
                                            .child(info.name),
                                    ),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x222228))
                                    .text_xs()
                                    .text_color(rgb(0xa1a1aa))
                                    .child(info.sector),
                            )
                    }))
            } else {
                div()
            },
        )
}
