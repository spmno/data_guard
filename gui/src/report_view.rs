//! 诊断报告的展示：文件摘要 + 逐列报告的虚拟列表。

use std::sync::Arc;

use data_guard::app::{DiagnosisReport, HealthScore, human_size};
use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::{badge, caption, health_color, stat_bar};

/// 报告视图：摘要 + 列表。
pub fn report_view(report: Signal<Option<Arc<DiagnosisReport>>>) -> impl IntoElement {
    view()
        .flex_1()
        .flex_col()
        .p_6()
        .gap_4()
        .child(summary(report))
        .child(column_list(report))
}

/// 文件摘要：文件名 + 总体评级徽章、路径、行/列/大小。
fn summary(report: Signal<Option<Arc<DiagnosisReport>>>) -> impl IntoElement {
    view()
        .flex_col()
        .gap_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .child(
                    text(move || {
                        report.with(|r| {
                            r.as_ref()
                                .map(|r| {
                                    r.file_info
                                        .path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "—".to_string())
                                })
                                .unwrap_or_else(|| "—".to_string())
                        })
                    })
                    .text_size(px(20.0))
                    .font_weight(FontWeight::SEMIBOLD),
                )
                .child(badge(move || {
                    report
                        .with(|r| r.as_ref().map(|r| r.overall_score))
                        .unwrap_or(HealthScore::Good)
                })),
        )
        .child(caption(move || {
            report.with(|r| {
                r.as_ref()
                    .map(|r| r.file_info.path.display().to_string())
                    .unwrap_or_default()
            })
        }))
        .child(caption(move || {
            report.with(|r| {
                r.as_ref()
                    .map(|r| {
                        format!(
                            "{} 行（扫描 {}） · {} 列 · {} · {} 列危险 / {} 列警告",
                            r.file_info.row_count,
                            r.scanned_rows,
                            r.file_info.column_count,
                            human_size(r.file_info.file_size),
                            r.danger_columns,
                            r.warning_columns,
                        )
                    })
                    .unwrap_or_default()
            })
        }))
}

/// 逐列报告的虚拟列表（列数可达数百，虚拟化保证滚动性能）。
fn column_list(report: Signal<Option<Arc<DiagnosisReport>>>) -> impl IntoElement {
    virtual_list(
        move || report.with(|r| r.as_ref().map_or(0, |r| r.column_reports.len())),
        move |at| column_row(report, at),
    )
    .flex_1()
}

/// 单列报告行：名称与类型、空值/重复徽章、两条比率条、建议。
fn column_row(report: Signal<Option<Arc<DiagnosisReport>>>, at: Position) -> impl IntoElement {
    // 当前行的 ColumnReport（虚拟列表的 cell 会被复用，内容以绑定读取）
    let column = move || {
        report.with(|r| {
            r.as_ref()
                .and_then(|r| r.column_reports.get(at.index()).cloned())
        })
    };

    view()
        .p_4()
        .flex_col()
        .gap_2()
        .border_b(px(1.0))
        .border_color(move || theme().border)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || column().map_or_else(String::new, |c| c.name))
                        .text_size(px(14.0))
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(caption(move || {
                    column().map_or_else(String::new, |c| c.data_type)
                }))
                .child(view().flex_1())
                .child(badge(move || {
                    column()
                        .map(|c| c.health_status.null_status)
                        .unwrap_or(HealthScore::Good)
                }))
                .child(badge(move || {
                    column()
                        .map(|c| c.health_status.duplicate_status)
                        .unwrap_or(HealthScore::Good)
                })),
        )
        .child(
            view()
                .flex_row()
                .gap_6()
                .child(stat_group(
                    "空值率",
                    move || column().map_or(0.0, |c| c.null_rate),
                    move || {
                        column()
                            .map(|c| c.health_status.null_status)
                            .unwrap_or(HealthScore::Good)
                    },
                ))
                .child(stat_group(
                    "重复率",
                    move || column().map_or(0.0, |c| c.duplicate_rate),
                    move || {
                        column()
                            .map(|c| c.health_status.duplicate_status)
                            .unwrap_or(HealthScore::Good)
                    },
                )),
        )
        .child(caption(move || {
            column()
                .map(|c| format!("唯一值 {} · 非空 {} 值", c.unique_count, c.non_null_count))
                .unwrap_or_default()
        }))
        .child(suggestions(column))
}

/// 一条比率统计：标签 + 右侧彩色数值 + 比率条。
fn stat_group(
    label: &'static str,
    rate: impl Fn() -> f64 + Copy + 'static,
    score: impl Fn() -> HealthScore + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_1()
        .flex_col()
        .gap_1()
        .child(
            view()
                .flex_row()
                .items_baseline()
                .child(
                    text(label)
                        .text_size(px(12.0))
                        .text_color(move || theme().muted_foreground),
                )
                .child(view().flex_1())
                .child(
                    text(move || format!("{:.1}%", rate() * 100.0))
                        .text_size(px(13.0))
                        .text_color(move || health_color(score())),
                ),
        )
        .child(stat_bar(rate, score))
}

/// 修复建议（琥珀色提示文字）；无建议时不占位。
fn suggestions(
    column: impl Fn() -> Option<data_guard::app::ColumnReport> + Copy + 'static,
) -> impl IntoElement {
    view().flex_col().gap_1().child(dynamic(move || {
        column().map_or_else(Vec::new, |c| {
            c.suggestions
                .iter()
                .map(|s| {
                    text(s.clone()).text_size(px(12.0)).text_color(move || {
                        theme()
                            .color("warn")
                            .unwrap_or_else(|| theme().muted_foreground)
                    })
                })
                .collect::<Vec<_>>()
        })
    }))
}
