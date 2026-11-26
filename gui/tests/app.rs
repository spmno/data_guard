//! 无头端到端测试：真实引擎（无窗口），通过注入文件对话框的应答驱动完整流程
//! （按钮 → 对话框 → blocking 线程池诊断 → 报告渲染）。

use std::path::PathBuf;

use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

fn fixture(name: &str) -> Option<PathBuf> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(name);
    path.exists().then_some(path)
}

fn start() -> Headless {
    Headless::start(EngineConfig::new(Root::of::<data_guard_gui::App>())).expect("启动应用失败")
}

#[test]
fn shows_empty_state() {
    let mut app = start();
    app.wait_for("选择 Parquet 文件").expect("应显示空态引导");
    app.wait_for("Parquet 数据质量诊断")
        .expect("应显示应用副标题");
}

#[test]
fn diagnoses_file_end_to_end() {
    let Some(issues) = fixture("test_issues.parquet") else {
        return;
    };
    let mut app = start();
    app.wait_for("选择 Parquet 文件").expect("应显示空态引导");

    // 用户"选择"了问题数据文件
    app.on_pick_paths(vec![issues]).expect("注入对话框应答失败");
    app.press("选择 Parquet 文件").expect("按下按钮失败");

    // 报告出现：文件名、危险评级、空值率、重复率
    app.wait_for("test_issues.parquet").expect("应显示文件名");
    app.wait_for("危险").expect("应显示总体危险评级");
    app.wait_for("空值率").expect("应显示空值率统计");
    app.wait_for("42.0%").expect("content 列空值率应为 42.0%");
    app.wait_for("重复率").expect("应显示重复率统计");
    // id 列：300 个重复的 1 + 700 个唯一值 → 重复率 29.9%
    app.wait_for("29.9%").expect("id 列重复率应为 29.9%");
    app.wait_for("唯一值 701 · 非空 1000 值")
        .expect("id 列唯一值应为 701");
}

#[test]
fn cancelled_dialog_keeps_empty_state() {
    let mut app = start();
    app.wait_for("选择 Parquet 文件").expect("应显示空态引导");

    // 用户取消（不选择任何文件）
    app.on_pick_paths(Vec::new()).expect("注入取消应答失败");
    app.press("选择 Parquet 文件").expect("按下按钮失败");

    app.wait_for("选择一个 Parquet 文件开始诊断")
        .expect("取消后应停留在空态");
}

#[test]
fn invalid_file_shows_error() {
    let bad = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("Cargo.toml");
    let mut app = start();
    app.wait_for("选择 Parquet 文件").expect("应显示空态引导");

    app.on_pick_paths(vec![bad]).expect("注入对话框应答失败");
    app.press("选择 Parquet 文件").expect("按下按钮失败");

    app.wait_for("诊断失败").expect("应显示错误状态");
    app.wait_for("重新选择文件").expect("应提供重新选择入口");
}
