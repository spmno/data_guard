//! 基于仓库自带 fixture（test_healthy.parquet / test_issues.parquet）的端到端集成测试。
//!
//! fixture 由 `generate_test_data.py` 生成：
//! - test_healthy: 1000 行，三列全部唯一、无空值
//! - test_issues: id 列 300 个重复的 1 + 700 个唯一值；content 列 42% 空值

use std::path::PathBuf;

use data_guard::app::HealthScore;
use data_guard::diagnosis::DiagnosisEngine;
use data_guard::parser::ParquetReader;

fn fixture(name: &str) -> Option<PathBuf> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    path.exists().then(|| path)
}

fn diagnose(path: &std::path::Path, limit: Option<usize>) -> data_guard::app::DiagnosisReport {
    let reader = ParquetReader::open(path).expect("打开文件失败");
    let info = reader.file_info();
    let mut engine = DiagnosisEngine::new(reader.schema());
    for batch in reader.batch_iter(limit).expect("创建迭代器失败") {
        engine
            .process_batch(&batch.expect("读取批次失败"))
            .expect("处理批次失败");
    }
    engine.finish(info)
}

#[test]
fn healthy_fixture_is_all_good() {
    let Some(path) = fixture("test_healthy.parquet") else {
        return;
    };
    let report = diagnose(&path, None);

    assert_eq!(report.scanned_rows, 1000);
    assert_eq!(report.overall_score, HealthScore::Good);
    assert_eq!(report.danger_columns, 0);
    assert_eq!(report.warning_columns, 0);
    for col in &report.column_reports {
        assert!((col.null_rate - 0.0).abs() < 1e-9, "列 {}", col.name);
        assert!((col.duplicate_rate - 0.0).abs() < 1e-9, "列 {}", col.name);
        assert_eq!(col.unique_count, 1000, "列 {}", col.name);
        assert!(col.suggestions.is_empty(), "列 {}", col.name);
    }
}

#[test]
fn issues_fixture_reports_expected_problems() {
    let Some(path) = fixture("test_issues.parquet") else {
        return;
    };
    let report = diagnose(&path, None);

    assert_eq!(report.scanned_rows, 1000);
    assert_eq!(report.overall_score, HealthScore::Danger);

    // id 列: 300 个 "1" + 700 个唯一值 → 唯一值 701，重复 299/1000
    let id = report
        .column_reports
        .iter()
        .find(|c| c.name == "id")
        .expect("应有 id 列");
    assert_eq!(id.unique_count, 701);
    assert!((id.duplicate_rate - 0.299).abs() < 1e-9);
    assert_eq!(id.health_status.duplicate_status, HealthScore::Warning);

    // content 列: 420/1000 空值 → 42%，Danger
    let content = report
        .column_reports
        .iter()
        .find(|c| c.name == "content")
        .expect("应有 content 列");
    assert!((content.null_rate - 0.42).abs() < 1e-9);
    assert_eq!(content.non_null_count, 580);
    assert_eq!(content.health_status.null_status, HealthScore::Danger);
    assert!(content.suggestions.iter().any(|s| s.contains("删除此列")));

    // title 列: 1000 个唯一值，无空值
    let title = report
        .column_reports
        .iter()
        .find(|c| c.name == "title")
        .expect("应有 title 列");
    assert!((title.null_rate - 0.0).abs() < 1e-9);
    assert_eq!(title.unique_count, 1000);
}

#[test]
fn limit_only_scans_requested_rows() {
    let Some(path) = fixture("test_issues.parquet") else {
        return;
    };
    let report = diagnose(&path, Some(100));
    assert_eq!(report.scanned_rows, 100);
    // 只扫前 100 行：id 全是重复的 1 → 常量列
    let id = report
        .column_reports
        .iter()
        .find(|c| c.name == "id")
        .expect("应有 id 列");
    assert_eq!(id.unique_count, 1);
    assert_eq!(id.non_null_count, 100);
}

#[test]
fn report_json_is_well_formed() {
    let Some(path) = fixture("test_issues.parquet") else {
        return;
    };
    let report = diagnose(&path, None);
    let value: serde_json::Value = serde_json::to_value(&report).expect("报告应可序列化为 JSON");
    assert_eq!(value["file_info"]["row_count"], 1000);
    assert_eq!(value["overall_score"], "Danger");
    assert!(value["column_reports"].is_array());
    assert_eq!(value["column_reports"].as_array().unwrap().len(), 3);
}
