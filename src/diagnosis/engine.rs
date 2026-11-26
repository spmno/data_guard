//! 流式诊断引擎：逐批累积统计，可处理任意大小的文件。
//!
//! 核心流程：`DiagnosisEngine::new(schema)` → 逐批 `process_batch` → `finish` 汇总报告。
//!
//! 指标定义：
//! - **空值率** = 空值数 / 总行数
//! - **重复率** = 非空值中的重复数 / 非空值总数（即 `1 - 唯一率`）
//! - 去重基于格式化后的真实列值（通过 Arrow `ArrayFormatter`，支持所有数据类型）

use anyhow::{Result, anyhow, bail};
use arrow::array::Array;
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use arrow::util::display::{ArrayFormatter, FormatOptions};
use std::collections::HashSet;

use crate::app::{ColumnReport, DiagnosisReport, FileInfo, HealthScore, HealthStatus, TextStats};

/// 空值率阈值：低于此值为 Good，达到即为 Warning，超过 [`NULL_DANGER`] 为 Danger
pub const NULL_WARNING: f64 = 0.05;
/// 空值率 Danger 阈值
pub const NULL_DANGER: f64 = 0.30;
/// 重复率阈值：低于此值为 Good，达到即为 Warning，超过 [`DUPLICATE_DANGER`] 为 Danger
pub const DUPLICATE_WARNING: f64 = 0.10;
/// 重复率 Danger 阈值
pub const DUPLICATE_DANGER: f64 = 0.30;

/// 流式诊断引擎。
pub struct DiagnosisEngine {
    columns: Vec<ColumnAccumulator>,
    scanned_rows: usize,
}

impl DiagnosisEngine {
    /// 按文件 schema 初始化各列的统计累积器。
    pub fn new(schema: &Schema) -> Self {
        let columns = schema
            .fields()
            .iter()
            .map(|field| {
                ColumnAccumulator::new(field.name().to_string(), field.data_type().clone())
            })
            .collect();
        Self {
            columns,
            scanned_rows: 0,
        }
    }

    /// 处理一个数据批次，累积到各列统计中。
    pub fn process_batch(&mut self, batch: &RecordBatch) -> Result<()> {
        if batch.num_columns() != self.columns.len() {
            bail!(
                "批次列数({})与文件列数({})不一致，文件可能损坏",
                batch.num_columns(),
                self.columns.len()
            );
        }
        for (index, accumulator) in self.columns.iter_mut().enumerate() {
            accumulator.process(batch.column(index).as_ref())?;
        }
        self.scanned_rows += batch.num_rows();
        Ok(())
    }

    /// 消耗引擎，汇总所有批次的统计并生成最终报告。
    pub fn finish(self, file_info: FileInfo) -> DiagnosisReport {
        let column_reports: Vec<ColumnReport> = self
            .columns
            .into_iter()
            .map(ColumnAccumulator::finish)
            .collect();

        let danger_columns = column_reports
            .iter()
            .filter(|r| {
                r.health_status.null_status == HealthScore::Danger
                    || r.health_status.duplicate_status == HealthScore::Danger
            })
            .count();
        let warning_columns = column_reports
            .iter()
            .filter(|r| {
                r.health_status.null_status == HealthScore::Warning
                    || r.health_status.duplicate_status == HealthScore::Warning
            })
            .count();
        let overall_score = if danger_columns > 0 {
            HealthScore::Danger
        } else if warning_columns > 0 {
            HealthScore::Warning
        } else {
            HealthScore::Good
        };

        DiagnosisReport {
            overall_score,
            danger_columns,
            warning_columns,
            scanned_rows: self.scanned_rows,
            file_info,
            column_reports,
        }
    }
}

/// 单列的流式统计累积器。
struct ColumnAccumulator {
    name: String,
    data_type: DataType,
    is_text: bool,
    total_rows: usize,
    null_count: usize,
    /// 非空值的去重集合。注意：超大文件的高基数列会占用较多内存，
    /// 后续可替换为固定内存的哈希草图（见 README 路线图）。
    unique_values: HashSet<String>,
    duplicate_count: usize,
    text_total_len: usize,
    text_rows: usize,
    has_html: bool,
}

impl ColumnAccumulator {
    fn new(name: String, data_type: DataType) -> Self {
        let is_text = data_type.is_string();
        Self {
            name,
            is_text,
            data_type,
            total_rows: 0,
            null_count: 0,
            unique_values: HashSet::new(),
            duplicate_count: 0,
            text_total_len: 0,
            text_rows: 0,
            has_html: false,
        }
    }

    fn process(&mut self, array: &dyn Array) -> Result<()> {
        self.total_rows += array.len();
        self.null_count += array.null_count();
        if array.is_empty() {
            return Ok(());
        }

        let options = FormatOptions::default();
        let formatter = ArrayFormatter::try_new(array, &options)
            .map_err(|err| anyhow!("列 {}({}) 的值无法解析: {err}", self.name, self.data_type))?;

        for i in 0..array.len() {
            if array.is_null(i) {
                continue;
            }
            // 真实列值（格式化后的字符串表示）参与去重与文本统计
            let value = formatter.value(i).to_string();
            if !self.unique_values.insert(value.clone()) {
                self.duplicate_count += 1;
            }
            if self.is_text {
                self.text_rows += 1;
                // 字符数而非字节数：Utf8 的多字节字符（如中文）按字符统计
                self.text_total_len += value.chars().count();
                if !self.has_html && looks_like_html(&value) {
                    self.has_html = true;
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> ColumnReport {
        // 非空数由 total_rows - null_count 推导。曾有版本用独立字段累加但
        // 忘了写入，恒为 0，连带重复率的分母为 0（重复率恒 0.0）。
        let non_null_count = self.total_rows - self.null_count;
        let null_rate = rate(self.null_count, self.total_rows);
        let duplicate_rate = rate(self.duplicate_count, non_null_count);
        let unique_count = self.unique_values.len();
        let health_status = determine_health_status(null_rate, duplicate_rate);
        let text_stats = if self.is_text {
            Some(TextStats {
                avg_length: rate(self.text_total_len, self.text_rows),
                has_html: self.has_html,
            })
        } else {
            None
        };
        let suggestions = generate_suggestions(
            null_rate,
            duplicate_rate,
            unique_count,
            non_null_count,
            self.has_html,
        );

        ColumnReport {
            name: self.name,
            data_type: self.data_type.to_string(),
            null_rate,
            duplicate_rate,
            unique_count,
            non_null_count,
            health_status,
            suggestions,
            text_stats,
        }
    }
}

/// 安全的除法，分母为 0 时返回 0.0（避免 NaN）。
fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn determine_health_status(null_rate: f64, duplicate_rate: f64) -> HealthStatus {
    let null_status = if null_rate < NULL_WARNING {
        HealthScore::Good
    } else if null_rate < NULL_DANGER {
        HealthScore::Warning
    } else {
        HealthScore::Danger
    };

    let duplicate_status = if duplicate_rate < DUPLICATE_WARNING {
        HealthScore::Good
    } else if duplicate_rate < DUPLICATE_DANGER {
        HealthScore::Warning
    } else {
        HealthScore::Danger
    };

    HealthStatus {
        null_status,
        duplicate_status,
    }
}

fn generate_suggestions(
    null_rate: f64,
    duplicate_rate: f64,
    unique_count: usize,
    non_null_count: usize,
    has_html: bool,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    if null_rate > NULL_DANGER {
        suggestions.push(format!(
            "空值率 {p:.1}%，建议删除此列或填充缺失值",
            p = null_rate * 100.0
        ));
    } else if null_rate >= NULL_WARNING {
        suggestions.push(format!(
            "空值率 {p:.1}%，建议检查空值分布",
            p = null_rate * 100.0
        ));
    }

    if non_null_count > 1 && unique_count == 1 {
        suggestions.push("整列只有 1 个唯一值（常量列），建议删除".to_string());
    } else if duplicate_rate > DUPLICATE_DANGER {
        suggestions.push(format!(
            "重复率 {p:.1}%，建议去重并核查数据来源",
            p = duplicate_rate * 100.0
        ));
    } else if duplicate_rate >= DUPLICATE_WARNING {
        suggestions.push(format!(
            "重复率 {p:.1}%，建议关注重复数据",
            p = duplicate_rate * 100.0
        ));
    }

    if has_html {
        suggestions.push("检测到 HTML 标签，建议清洗后再用于训练".to_string());
    }

    suggestions
}

/// 简易 HTML 启发式：同时出现 `<` 和 `>`。
fn looks_like_html(value: &str) -> bool {
    value.contains('<') && value.contains('>')
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{ArrayRef, Int64Array, StringArray};
    use arrow::datatypes::{Field, Schema};
    use std::sync::Arc;

    fn file_info() -> FileInfo {
        FileInfo {
            path: "test.parquet".into(),
            row_count: 0,
            column_count: 0,
            file_size: 0,
        }
    }

    fn string_batch(values: Vec<Option<&str>>) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("col", DataType::Utf8, true)]));
        let array = StringArray::from(values);
        RecordBatch::try_new(schema, vec![Arc::new(array) as ArrayRef]).expect("构造批次失败")
    }

    fn int_batch(values: Vec<Option<i64>>) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("col", DataType::Int64, true)]));
        let array = Int64Array::from(values);
        RecordBatch::try_new(schema, vec![Arc::new(array) as ArrayRef]).expect("构造批次失败")
    }

    fn run(batches: &[RecordBatch]) -> DiagnosisReport {
        assert!(!batches.is_empty());
        let mut engine = DiagnosisEngine::new(&batches[0].schema());
        for batch in batches {
            engine.process_batch(batch).expect("处理批次失败");
        }
        engine.finish(file_info())
    }

    fn first_column(report: &DiagnosisReport) -> &ColumnReport {
        &report.column_reports[0]
    }

    #[test]
    fn healthy_column() {
        let batches = vec![string_batch(vec![Some("a"), Some("b"), Some("c")])];
        let report = run(&batches);
        let col = first_column(&report);
        assert_eq!(col.null_rate, 0.0);
        assert_eq!(col.duplicate_rate, 0.0);
        assert_eq!(col.unique_count, 3);
        assert_eq!(col.non_null_count, 3);
        assert_eq!(col.health_status.null_status, HealthScore::Good);
        assert_eq!(col.health_status.duplicate_status, HealthScore::Good);
        assert!(col.suggestions.is_empty());
        assert_eq!(report.overall_score, HealthScore::Good);
        assert_eq!(report.scanned_rows, 3);
    }

    #[test]
    fn high_null_rate_is_danger() {
        let batches = vec![string_batch(vec![Some("a"), None, None])];
        let report = run(&batches);
        let col = first_column(&report);
        assert!((col.null_rate - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(col.health_status.null_status, HealthScore::Danger);
        assert_eq!(col.non_null_count, 1);
        // 只有一个非空值时谈不上重复
        assert_eq!(col.duplicate_rate, 0.0);
        assert!(col.suggestions.iter().any(|s| s.contains("删除此列")));
    }

    /// 回归测试：重复率必须基于真实列值（曾用占位符导致重复率恒为 0）
    #[test]
    fn duplicate_rate_uses_real_values() {
        let batches = vec![string_batch(vec![
            Some("a"),
            Some("a"),
            Some("b"),
            Some("b"),
        ])];
        let report = run(&batches);
        let col = first_column(&report);
        assert!((col.duplicate_rate - 0.5).abs() < 1e-9);
        assert_eq!(col.unique_count, 2);
        assert_eq!(col.health_status.duplicate_status, HealthScore::Danger);
        assert!(col.suggestions.iter().any(|s| s.contains("重复率")));
    }

    #[test]
    fn numeric_column_duplicates() {
        let batches = vec![int_batch(vec![Some(1), Some(1), Some(2)])];
        let report = run(&batches);
        let col = first_column(&report);
        assert!((col.duplicate_rate - 1.0 / 3.0).abs() < 1e-9);
        assert_eq!(col.unique_count, 2);
        assert!(col.text_stats.is_none());
    }

    #[test]
    fn detects_html_tags() {
        let batches = vec![string_batch(vec![Some("<b>bold</b>"), Some("plain text")])];
        let report = run(&batches);
        let col = first_column(&report);
        let text = col.text_stats.as_ref().expect("文本列应有 text_stats");
        assert!(text.has_html);
        // "<b>bold</b>" 11 字符 + "plain text" 10 字符 → 平均 10.5
        assert!((text.avg_length - 10.5).abs() < 1e-9);
        assert!(col.suggestions.iter().any(|s| s.contains("HTML")));
    }

    #[test]
    fn constant_column_suggests_removal() {
        let batches = vec![string_batch(vec![Some("x"), Some("x"), Some("x")])];
        let report = run(&batches);
        let col = first_column(&report);
        assert_eq!(col.unique_count, 1);
        assert!((col.duplicate_rate - 2.0 / 3.0).abs() < 1e-9);
        assert!(col.suggestions.iter().any(|s| s.contains("常量列")));
    }

    #[test]
    fn empty_batch_yields_zero_rates() {
        let batches = vec![string_batch(Vec::new())];
        let report = run(&batches);
        let col = first_column(&report);
        assert_eq!(col.null_rate, 0.0);
        assert_eq!(col.duplicate_rate, 0.0);
        assert_eq!(col.unique_count, 0);
        assert!(col.null_rate.is_finite() && col.duplicate_rate.is_finite());
        assert_eq!(report.overall_score, HealthScore::Good);
    }

    #[test]
    fn all_null_column() {
        let batches = vec![string_batch(vec![None, None])];
        let report = run(&batches);
        let col = first_column(&report);
        assert_eq!(col.null_rate, 1.0);
        assert_eq!(col.health_status.null_status, HealthScore::Danger);
        assert_eq!(col.non_null_count, 0);
        assert_eq!(col.duplicate_rate, 0.0);
        assert_eq!(col.unique_count, 0);
    }

    #[test]
    fn stats_aggregate_across_batches() {
        let batch1 = string_batch(vec![Some("a"), Some("b")]);
        let batch2 = string_batch(vec![Some("a"), Some("c")]);
        let report = run(&[batch1, batch2]);
        let col = first_column(&report);
        assert_eq!(report.scanned_rows, 4);
        assert_eq!(col.unique_count, 3);
        assert_eq!(col.non_null_count, 4);
        assert!((col.duplicate_rate - 0.25).abs() < 1e-9);
        assert_eq!(col.health_status.duplicate_status, HealthScore::Warning);
        let text = col.text_stats.as_ref().expect("应有文本统计");
        assert!((text.avg_length - 1.0).abs() < 1e-9);
    }

    #[test]
    fn column_count_mismatch_is_an_error() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("a", DataType::Utf8, true),
            Field::new("b", DataType::Utf8, true),
        ]));
        let batch = string_batch(vec![Some("a")]);
        let mut engine = DiagnosisEngine::new(&schema);
        assert!(engine.process_batch(&batch).is_err());
    }

    #[test]
    fn report_serializes_to_json() {
        let batches = vec![string_batch(vec![Some("a"), None])];
        let report = run(&batches);
        let json = serde_json::to_string(&report).expect("序列化失败");
        assert!(json.contains("\"overall_score\""));
        assert!(json.contains("\"column_reports\""));
        assert!(json.contains("\"null_rate\""));
    }
}
