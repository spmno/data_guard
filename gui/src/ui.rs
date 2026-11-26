//! 应用的样式组件层（ui layer）。
//!
//! 所有颜色、圆角、字号集中在这里，页面代码（`lib.rs` / `report_view.rs`）
//! 只组合这些组件，不直接写视觉样式。颜色一律取自主题 token（`theme()`），
//! 健康等级的语义色来自 `theme.json` 的 `good` / `warn` 与主题的 `destructive`。

use data_guard::app::HealthScore;
use nsh_base::theme;
use nsh_core::prelude::*;

/// 主操作按钮：基于 `nsh_base::button` 的行为（焦点、键盘、无障碍），
/// 外观为主题主色。调用方再链上 `.on_press(...)`。
pub fn primary_button(label: &'static str) -> nsh_base::ButtonElement {
    nsh_base::button()
        .px(px(20.0))
        .py(px(10.0))
        .rounded(move || theme().radius)
        .transition()
        .bg(move || theme().primary)
        .active(|s| s.opacity(0.85))
        .child(
            text(label)
                .text_size(px(14.0))
                .text_color(move || theme().primary_foreground),
        )
}

/// 次要按钮（弱化外观）。
pub fn secondary_button(label: &'static str) -> nsh_base::ButtonElement {
    nsh_base::button()
        .px(px(16.0))
        .py(px(8.0))
        .rounded(move || theme().radius)
        .transition()
        .bg(move || theme().secondary)
        .active(|s| s.opacity(0.85))
        .child(
            text(label)
                .text_size(px(13.0))
                .text_color(move || theme().secondary_foreground),
        )
}

/// 健康等级徽章：同色系浅底（12% 透明度）+ 同色文字，颜色变化有过渡。
pub fn badge(score: impl Fn() -> HealthScore + Copy + 'static) -> impl IntoElement {
    view()
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(999.0))
        .transition()
        .bg(move || health_color(score()).opacity(0.12))
        .child(
            text(move || health_label(score()))
                .text_size(px(12.0))
                .text_color(move || health_color(score())),
        )
}

/// 比率条：4px 轨道（muted）+ 健康色填充，宽度为 `rate` 的百分比。
pub fn stat_bar(
    rate: impl Fn() -> f64 + Copy + 'static,
    score: impl Fn() -> HealthScore + Copy + 'static,
) -> impl IntoElement {
    view()
        .h(px(4.0))
        .w_full()
        .rounded(px(2.0))
        .bg(move || theme().muted)
        .child(
            view()
                .h_full()
                .rounded(px(2.0))
                .transition()
                .bg(move || health_color(score()).opacity(0.9))
                .w(move || {
                    Length::Definite(DefiniteLength::Fraction(Percentage(
                        rate().clamp(0.0, 1.0) as f32
                    )))
                }),
        )
}

/// 健康等级的中文名。
pub fn health_label(score: HealthScore) -> SharedString {
    match score {
        HealthScore::Good => "良好".into(),
        HealthScore::Warning => "警告".into(),
        HealthScore::Danger => "危险".into(),
    }
}

/// 健康等级的语义色：good / warn 取自主题自定义 token，危险用 `destructive`。
pub fn health_color(score: HealthScore) -> AdaptiveColor {
    match score {
        HealthScore::Good => theme()
            .color("good")
            .unwrap_or_else(|| theme().muted_foreground),
        HealthScore::Warning => theme()
            .color("warn")
            .unwrap_or_else(|| theme().muted_foreground),
        HealthScore::Danger => theme().destructive,
    }
}

/// 辅助文字样式（12px muted），用于标注性文本。
pub fn caption(content: impl IntoReactive<SharedString>) -> nsh_core::element::Text {
    text(content)
        .text_size(px(12.0))
        .text_color(move || theme().muted_foreground)
}
