//! data_guard 图形界面：NSH（Native Shell）纯 Rust 应用。
//!
//! 结构：
//! - [`App`]：根组件，持有全局状态（文件、报告、错误、扫描中）
//! - [`ui`]：样式组件层（按钮、徽章、比率条）
//! - [`report_view`]：报告展示（摘要 + 虚拟列表）
//!
//! 诊断在 `blocking` 线程池执行（`data_guard::diagnosis::diagnose_file`），
//! 文件选择走系统对话框（`pick_paths`）。

use std::sync::Arc;

use data_guard::app::DiagnosisReport;
use data_guard::diagnosis::diagnose_file;
use nsh_base::theme;
use nsh_core::file_dialog::{FileFilter, OpenDialog};
use nsh_core::prelude::*;

pub mod report_view;
pub mod ui;

pub use report_view::report_view;

/// 根组件在 TSX 模式下可导入的 UI 集（`nsh.json` `"rust"`）。
#[native_module("data_guard/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

/// 全局界面状态：四个信号的 `Copy` 句柄，跨闭包共享。
#[derive(Clone, Copy)]
pub struct GuiState {
    /// 当前文件路径（展示用）
    pub file: Signal<Option<SharedString>>,
    /// 最新诊断报告
    pub report: Signal<Option<Arc<DiagnosisReport>>>,
    /// 错误信息
    pub error: Signal<Option<SharedString>>,
    /// 是否正在扫描
    pub scanning: Signal<bool>,
    /// App 根组件的 owner，与根组件同寿命。
    ///
    /// 诊断任务必须挂在它下面：任务会归属 `spawn` 时的当前 owner，而
    /// 空态/错误态的按钮在 `dynamic` 内容里——从那里直接 `spawn`，任务
    /// 归瞬态内容的 owner 所有，内容切换（空态 → 扫描态）时任务被连带
    /// 取消，界面就永远停在"正在扫描…"。
    owner: Owner,
}

/// 应用根组件。
#[component]
pub fn App() -> impl IntoElement {
    let state = GuiState {
        file: signal(None),
        report: signal(None),
        error: signal(None),
        scanning: signal(false),
        owner: Owner::current(),
    };

    view()
        .flex_1()
        .flex_col()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(header(state))
        .child(content(state))
}

/// 顶栏：应用名 + 当前文件 + 选择文件按钮。
fn header(state: GuiState) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .px_6()
        .py_4()
        .border_b(px(1.0))
        .border_color(move || theme().border)
        .child(
            text("data_guard")
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .child(ui::caption("Parquet 数据质量诊断"))
        .child(view().flex_1())
        .child(dynamic(move || {
            state
                .file
                .get()
                .map(|file| ui::caption(move || file.clone()).truncate().into_element())
        }))
        .child(ui::primary_button("选择文件").on_press(move || pick_and_diagnose(state)))
}

/// 主内容区：扫描中 / 错误 / 报告 / 空态 四选一。
fn content(state: GuiState) -> impl IntoElement {
    view().flex_1().flex_col().min_h_0().child(dynamic(move || {
        if state.scanning.get() {
            scanning_view(state).into_element()
        } else if state.error.get().is_some() {
            error_view(state).into_element()
        } else if state.report.get().is_some() {
            report_view(state.report).into_element()
        } else {
            empty_view(state).into_element()
        }
    }))
}

/// 空态：引导选择文件。
fn empty_view(state: GuiState) -> AnyElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .child(
            view()
                .flex_col()
                .items_center()
                .gap_3()
                .px_10()
                .py_10()
                .rounded(move || theme().radius)
                .bg(move || theme().card)
                .child(
                    text("选择一个 Parquet 文件开始诊断")
                        .text_size(px(15.0))
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(ui::caption("空值率 · 重复率 · 唯一值 · 修复建议"))
                .child(
                    ui::primary_button("选择 Parquet 文件")
                        .on_press(move || pick_and_diagnose(state)),
                ),
        )
        .into_element()
}

/// 扫描中：文件路径 + 提示。
fn scanning_view(state: GuiState) -> AnyElement {
    view()
        .flex_1()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            text("正在扫描…")
                .text_size(px(15.0))
                .font_weight(FontWeight::MEDIUM),
        )
        .child(ui::caption(move || {
            state.file.get().map(|f| f.to_string()).unwrap_or_default()
        }))
        .into_element()
}

/// 错误态：错误信息 + 重新选择。
fn error_view(state: GuiState) -> AnyElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .child(
            view()
                .flex_col()
                .items_center()
                .gap_3()
                .px_10()
                .py_10()
                .rounded(move || theme().radius)
                .bg(move || theme().destructive.opacity(0.08))
                .child(
                    text("诊断失败")
                        .text_size(px(15.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || theme().destructive),
                )
                .child(ui::caption(move || {
                    state.error.get().map(|e| e.to_string()).unwrap_or_default()
                }))
                .child(
                    ui::secondary_button("重新选择文件").on_press(move || pick_and_diagnose(state)),
                ),
        )
        .into_element()
}

fn pick_and_diagnose(state: GuiState) {
    let dialog = pick_paths(
        OpenDialog::files()
            .filter(FileFilter::new("Parquet", ["parquet"]))
            .button_label("诊断"),
    );
    // 挂在根组件的 owner 下：任务与 App 同寿命，内容切换不会取消它
    state.owner.run(|| {
        spawn(async move {
            match dialog.await {
                Ok(Some(paths)) => {
                    let Some(path) = paths.into_iter().next() else {
                        return;
                    };
                    state.file.set(Some(path.display().to_string().into()));
                    state.scanning.set(true);
                    state.error.set(None);
                    state.report.set(None);
                    let result = blocking(move || diagnose_file(&path, None)).await;
                    state.scanning.set(false);
                    match result {
                        Ok(Ok(report)) => state.report.set(Some(Arc::new(report))),
                        Ok(Err(err)) => state.error.set(Some(format!("{err:#}").into())),
                        Err(err) => state.error.set(Some(format!("扫描任务失败: {err}").into())),
                    }
                }
                // 用户取消选择，界面保持原状
                Ok(None) => {}
                Err(err) => state.error.set(Some(err.into())),
            }
        });
    });
}
