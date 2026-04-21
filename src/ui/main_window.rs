use gpui::{
    actions, div, prelude::*, px, rgb, size, App, AppContext, Bounds, Color, Context,
    SharedString, Stateful, Window, WindowBounds, WindowOptions,
};
use crate::app::{AppState, ColumnReport, DiagnosisReport, FileInfo, HealthScore};

actions!(data_guard, [OpenFile, RunDiagnosis]);

// ─── App State ────────────────────────────────────────────────────────────────

pub struct DataGuardApp {
    state: AppState,
    loading: bool,
    error_message: Option<SharedString>,
}

impl DataGuardApp {
    fn open_file(&mut self, window: &mut Window, cx: &mut AppContext) {
        cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open Parquet File".into()),
        })
        .unwrap_or_else(|| Vec::new());
    }

    fn run_diagnosis(&mut self, file_path: std::path::PathBuf, cx: &mut AppContext) {
        self.loading = true;
        self.error_message = None;
        cx.notify();

        let app_state = cx.app_state().clone();
        cx.spawn_in(window, async move |_, cx| {
            let result = tokio::task::spawn_blocking(move || {
                run_diagnosis_sync(file_path)
            }).await;

            match result {
                Ok(Ok(report)) => {
                    cx.update(|cx| {
                        let app = cx.state::<DataGuardApp>();
                        app.state.diagnosis_report = Some(report);
                        app.loading = false;
                        cx.notify();
                    }).ok();
                }
                Ok(Err(e)) => {
                    cx.update(|cx| {
                        let app = cx.state::<DataGuardApp>();
                        app.error_message = Some(e.to_string().into());
                        app.loading = false;
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    cx.update(|cx| {
                        let app = cx.state::<DataGuardApp>();
                        app.error_message = Some(format!("Task error: {}", e).into());
                        app.loading = false;
                        cx.notify();
                    }).ok();
                }
            }
        });
    }
}

fn run_diagnosis_sync(file_path: std::path::PathBuf) -> anyhow::Result<DiagnosisReport> {
    use crate::parser::ParquetReader;
    use crate::diagnosis::diagnose_batch;
    use crate::app::{FileInfo, HealthScore};

    let mut reader = crate::parser::ParquetReader::new(file_path.clone())?;
    let (row_count, column_count, file_size) = reader.get_file_info()?;

    let file_info = FileInfo {
        path: file_path.clone(),
        row_count,
        column_count,
        file_size,
    };

    let batches = reader.read_batches(1000)?;
    let mut all_column_reports = Vec::new();

    for batch in batches {
        let column_reports = diagnose_batch(&batch);
        all_column_reports.extend(column_reports);
    }

    // Compute overall score
    let overall_score = if all_column_reports.iter().all(|r| {
        r.health_status.null_status == HealthScore::Good
        && r.health_status.duplicate_status == HealthScore::Good
    }) {
        HealthScore::Good
    } else if all_column_reports.iter().any(|r| {
        r.health_status.null_status == HealthScore::Danger
        || r.health_status.duplicate_status == HealthScore::Danger
    }) {
        HealthScore::Danger
    } else {
        HealthScore::Warning
    };

    Ok(DiagnosisReport {
        overall_score,
        file_info,
        column_reports: all_column_reports,
    })
}

impl Render for DataGuardApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_report = self.state.diagnosis_report.is_some();
        let error_msg = self.error_message.clone();
        let loading = self.loading;
        let file_path = self.state.file_path.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .on_action(cx.listener(|this, _: &OpenFile, window, cx| {
                this.open_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &RunDiagnosis, window, cx| {
                if let Some(ref path) = this.state.file_path {
                    this.run_diagnosis(path.clone(), cx);
                }
            }))
            .children([
                // ── Top Bar ─────────────────────────────────────────────────
                top_bar(cx),

                // ── Content Area ──────────────────────────────────────────────
                if loading {
                    loading_view()
                } else if let Some(ref msg) = error_msg {
                    error_view(msg.clone())
                } else if has_report {
                    report_view(self.state.diagnosis_report.as_ref().unwrap())
                } else {
                    empty_state_view(cx)
                },
            ])
    }
}

// ─── Top Bar ─────────────────────────────────────────────────────────────────

fn top_bar(cx: &mut Context<DataGuardApp>) -> Stateful<gpui::Div> {
    div()
        .flex()
        .items_center()
        .h(px(52.0))
        .px_4()
        .bg(rgb(0x181825))
        .border_b_1()
        .border_color(rgb(0x313244))
        .child(
            div()
                .flex_1()
                .text_xl()
                .font_semibold()
                .text_color(rgb(0xcdd6f4))
                .child("🛡️ DataGuard"),
        )
        .child(
            gpui::button("Open File")
                .px_4()
                .py_2()
                .rounded_md()
                .bg(rgb(0x89b4fa))
                .text_color(rgb(0x1e1e2e))
                .on_click(cx.listener(|this, _, window, cx| {
                    cx.prompt_for_paths(PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: false,
                        prompt: Some("Open Parquet File".into()),
                    })
                    .map(|paths| {
                        if let Some(path) = paths.into_iter().next() {
                            this.state.file_path = Some(path.clone());
                            this.run_diagnosis(path, cx);
                        }
                    });
                })),
        )
}

// ─── Empty State ─────────────────────────────────────────────────────────────

fn empty_state_view(cx: &mut Context<DataGuardApp>) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_4()
        .child(
            gpui::svg::Svg::new()
                .path("M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z")
                .size(px(80.0))
                .text_color(rgb(0x6c7086)),
        )
        .child(
            div()
                .text_2xl()
                .font_medium()
                .text_color(rgb(0x6c7086))
                .child("Drop a Parquet file or click Open File"),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x45475a))
                .child("Supports .parquet files up to 10GB"),
        )
}

// ─── Loading ─────────────────────────────────────────────────────────────────

fn loading_view() -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_4()
        .child(div().text_xl().text_color(rgb(0x89b4fa)).child("Analyzing..."))
}

// ─── Error ────────────────────────────────────────────────────────────────────

fn error_view(msg: SharedString) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_4()
        .child(div().text_xl().text_color(rgb(0xf38ba8)).child("⚠️ Error"))
        .child(
            div()
                .max_w(px(600.0))
                .text_sm()
                .text_color(rgb(0xf5c2e7))
                .child(msg),
        )
}

// ─── Diagnosis Report ─────────────────────────────────────────────────────────

fn report_view(report: &DiagnosisReport) -> impl IntoElement {
    let score_color = match report.overall_score {
        HealthScore::Good => rgb(0xa6e3a1),
        HealthScore::Warning => rgb(0xf9e2af),
        HealthScore::Danger => rgb(0xf38ba8),
    };
    let score_label = match report.overall_score {
        HealthScore::Good => "✅ Healthy",
        HealthScore::Warning => "⚠️ Warning",
        HealthScore::Danger => "❌ Issues Found",
    };

    let file_size_mb = report.file_info.file_size as f64 / 1_048_576.0;

    div()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden()
        // ── File Info Bar ────────────────────────────────────────────────
        .child(
            div()
                .flex()
                .items_center()
                .gap_6()
                .px_4()
                .py_3()
                .bg(rgb(0x181825))
                .border_b_1()
                .border_color(rgb(0x313244))
                .children([
                    label_item("📄", report.file_info.path.file_name().and_then(|s| s.to_str()).unwrap_or("?")),
                    label_item("Rows", &format!("{:?}", report.file_info.row_count)),
                    label_item("Cols", &report.file_info.column_count.to_string()),
                    label_item("Size", &format!("{:.1} MB", file_size_mb)),
                    div().flex_1(),
                    div().text_base().font_semibold().text_color(score_color).child(score_label),
                ]),
        )
        // ── Column Reports ───────────────────────────────────────────────
        .child(
            div()
                .flex_1()
                .overflow_auto()
                .px_4()
                .py_3()
                .flex()
                .flex_col()
                .gap_3()
                .children(report.column_reports.iter().map(|col| column_card(col))),
        )
}

fn label_item(icon: &str, value: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .text_sm()
        .text_color(rgb(0xa6adc8))
        .children([
            gpui::label(icon).text_sm(),
            gpui::label(value).font_medium(),
        ])
}

fn column_card(col: &ColumnReport) -> impl IntoElement {
    let null_color = health_color(&col.health_status.null_status);
    let dup_color = health_color(&col.health_status.duplicate_status);
    let has_issues = col.health_status.null_status != HealthScore::Good
        || col.health_status.duplicate_status != HealthScore::Good;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_lg()
        .bg(rgb(0x181825))
        .border_1()
        .border_color(if has_issues { rgb(0x45475a) } else { rgb(0x313244) })
        .when(has_issues, |d| {
            d.border_1().border_color(rgb(0xf9e2af).opacity(0.3))
        })
        .children([
            // Column header
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(gpui::label(&col.name).font_semibold().text_color(rgb(0xcdd6f4))),
            // Stats row
            div()
                .flex()
                .gap_4()
                .children([
                    stat_badge("Nulls", &format!("{:.1}%", col.null_rate * 100.0), null_color),
                    stat_badge("Dupes", &format!("{:.1}%", col.duplicate_rate * 100.0), dup_color),
                    stat_badge("Unique", &col.unique_count.to_string(), rgb(0xa6adc8)),
                ]),
            // Text stats if string column
            col.text_stats.as_ref().map(|ts| {
                div()
                    .flex()
                    .gap_4()
                    .children([
                        stat_badge("AvgLen", &format!("{:.0}", ts.avg_length), rgb(0xa6adc8)),
                        if ts.has_html {
                            stat_badge("HTML", "Yes", rgb(0xf9e2af))
                        } else {
                            div()
                        },
                    ])
                    .mt_1()
            }),
            // Suggestions
            if !col.suggestions.is_empty() {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .mt_1()
                    .children(col.suggestions.iter().map(|s| {
                        div()
                            .text_xs()
                            .text_color(rgb(0xf9e2af))
                            .child(format!("💡 {}", s))
                    }))
            } else {
                div()
            },
        ])
}

fn stat_badge(label: &str, value: &str, color: Color) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(color.opacity(0.12))
        .text_xs()
        .text_color(color)
        .child(gpui::label(label).opacity(0.7))
        .child(gpui::label(value).font_medium())
}

fn health_color(score: &HealthScore) -> Color {
    match score {
        HealthScore::Good => rgb(0xa6e3a1),
        HealthScore::Warning => rgb(0xf9e2af),
        HealthScore::Danger => rgb(0xf38ba8),
    }
}

// ─── Module Entry ──────────────────────────────────────────────────────────────

pub fn run_gui() {
    App::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(
            None,
            size(px(1100.0), px(750.0)),
            cx,
        );
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::WindowTitlebar {
                    title: Some("DataGuard — AI Data Quality Gatekeeper".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| DataGuardApp {
                    state: AppState::new(),
                    loading: false,
                    error_message: None,
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
