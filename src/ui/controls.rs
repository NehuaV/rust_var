use gpui::*;
use crate::risk::OptMethod;
use super::app_view::AppView;

pub fn render_controls(
    view: &mut AppView,
    _window: &mut Window,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let cur_conf = view.state.confidence_level;
    let cur_days = view.state.days;
    let cur_opt = view.state.opt_method;
    let is_busy = view.state.is_busy;
    let is_calculating = view.state.is_calculating;

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
            div()
                .text_base()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0xf4f4f5))
                .child("Risk Modeling & Optimization Setup"),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap_6()
                .child(
                    // Confidence Level
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xa1a1aa))
                                .child("Confidence Level (1-tailed):"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_1()
                                .child(conf_button(view, 0.90, "90%", cur_conf, cx))
                                .child(conf_button(view, 0.95, "95%", cur_conf, cx))
                                .child(conf_button(view, 0.99, "99%", cur_conf, cx))
                                .child(conf_button(view, 0.999, "99.9%", cur_conf, cx)),
                        ),
                )
                .child(
                    // Horizon Days
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xa1a1aa))
                                .child("Horizon (Trading Days):"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_1()
                                .child(days_button(view, 1.0, "1d", cur_days, cx))
                                .child(days_button(view, 5.0, "5d (1w)", cur_days, cx))
                                .child(days_button(view, 10.0, "10d (2w)", cur_days, cx))
                                .child(days_button(view, 21.0, "21d (1m)", cur_days, cx)),
                        ),
                )
                .child(
                    // Optimization Method
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xa1a1aa))
                                .child("Optimization Objective:"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_1()
                                .child(opt_button(
                                    view,
                                    Some(OptMethod::Pgd),
                                    "PGD (Long-Only)",
                                    cur_opt,
                                    cx,
                                ))
                                .child(opt_button(
                                    view,
                                    Some(OptMethod::Gmv),
                                    "GMV (Unconstrained)",
                                    cur_opt,
                                    cx,
                                ))
                                .child(opt_button(view, None, "None (Plain VaR)", cur_opt, cx)),
                        ),
                ),
        )
        .child(
            // Primary Optimization Button
            div()
                .flex()
                .flex_row()
                .justify_between()
                .items_center()
                .pt_2()
                .border_t_1()
                .border_color(rgb(0x27272a))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xa1a1aa))
                        .child(match cur_opt {
                            Some(OptMethod::Pgd) => {
                                "Projected Gradient Descent ensures all allocations are non-negative (x >= 0) summing to 100%."
                            }
                            Some(OptMethod::Gmv) => {
                                "Global Minimum Variance analytical solution (short selling allowed)."
                            }
                            None => "Standard parametric portfolio VaR with no reallocation.",
                        }),
                )
                .child(
                    div()
                        .id("run-optimization-btn")
                        .px_6()
                        .py_2p5()
                        .rounded_md()
                        .bg(if is_busy || is_calculating || view.state.positions.is_empty() {
                            rgb(0x3f3f46)
                        } else {
                            rgb(0x16a34a)
                        })
                        .text_color(rgb(0xffffff))
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(if is_busy || is_calculating || view.state.positions.is_empty() {
                            rgb(0x3f3f46)
                        } else {
                            rgb(0x15803d)
                        }))
                        .on_click(cx.listener(|this, _event: &ClickEvent, _window, cx| {
                            this.trigger_optimize(cx);
                        }))
                        .child(if is_calculating {
                            "⚡ Calculating VaR in Background..."
                        } else {
                            "⚡ Calculate & Optimize Portfolio"
                        }),
                ),
        )
}

fn conf_button(
    _view: &mut AppView,
    level: f64,
    label: &'static str,
    current: f64,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let is_selected = (current - level).abs() < 1e-4;
    div()
        .id(ElementId::Name(format!("conf-btn-{}", label).into()))
        .px_3()
        .py_1p5()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .bg(if is_selected { rgb(0x2563eb) } else { rgb(0x27272a) })
        .text_color(if is_selected { rgb(0xffffff) } else { rgb(0xd4d4d8) })
        .hover(|s| s.bg(if is_selected { rgb(0x1d4ed8) } else { rgb(0x3f3f46) }))
        .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
            this.set_confidence_level(level, cx);
        }))
        .child(label)
}

fn days_button(
    _view: &mut AppView,
    days: f64,
    label: &'static str,
    current: f64,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let is_selected = (current - days).abs() < 1e-4;
    div()
        .id(ElementId::Name(format!("days-btn-{}", label).into()))
        .px_3()
        .py_1p5()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .bg(if is_selected { rgb(0x2563eb) } else { rgb(0x27272a) })
        .text_color(if is_selected { rgb(0xffffff) } else { rgb(0xd4d4d8) })
        .hover(|s| s.bg(if is_selected { rgb(0x1d4ed8) } else { rgb(0x3f3f46) }))
        .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
            this.set_horizon_days(days, cx);
        }))
        .child(label)
}

fn opt_button(
    _view: &mut AppView,
    method: Option<OptMethod>,
    label: &'static str,
    current: Option<OptMethod>,
    cx: &mut Context<AppView>,
) -> impl IntoElement {
    let is_selected = current == method;
    div()
        .id(ElementId::Name(format!("opt-btn-{}", label).into()))
        .px_3()
        .py_1p5()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .bg(if is_selected { rgb(0x7c3aed) } else { rgb(0x27272a) })
        .text_color(if is_selected { rgb(0xffffff) } else { rgb(0xd4d4d8) })
        .hover(|s| s.bg(if is_selected { rgb(0x6d28d9) } else { rgb(0x3f3f46) }))
        .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
            this.set_opt_method(method, cx);
        }))
        .child(label)
}
