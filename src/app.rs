//! 数据模型：诊断报告的结构定义。
//!
//! 所有报告类型都实现了 `Serialize`，便于以 JSON 输出接入数据流水线。

use serde::Serialize;
use std::path::PathBuf;

/// 应用全局状态（预留给 GUI 模式使用）。
#[derive(Debug, Default)]
pub struct AppState {
    pub file_path: Option<PathBuf>,
    pub file_info: Option<FileInfo>,
    pub diagnosis_report: Option<DiagnosisReport>,
}

/// Parquet 文件的基础信息。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub row_count: i64,
    pub column_count: usize,
    pub file_size: u64,
}

/// 整份文件的诊断报告。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiagnosisReport {
    /// 总体评级：任一列 Danger 则 Danger，否则任一列 Warning 则 Warning
    pub overall_score: HealthScore,
    /// 危险列数量
    pub danger_columns: usize,
    /// 警告列数量
    pub warning_columns: usize,
    /// 实际扫描的行数（受 `--limit` 影响时可能小于文件总行数）
    pub scanned_rows: usize,
    pub file_info: FileInfo,
    pub column_reports: Vec<ColumnReport>,
}

/// 单列的诊断报告。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ColumnReport {
    pub name: String,
    /// Arrow 数据类型（Display 格式，如 `Utf8`、`Int64`）
    pub data_type: String,
    /// 空值数 / 总行数，范围 [0, 1]
    pub null_rate: f64,
    /// 非空值中重复值的占比，范围 [0, 1]
    pub duplicate_rate: f64,
    /// 非空去重后的唯一值数量
    pub unique_count: usize,
    /// 非空值数量
    pub non_null_count: usize,
    pub health_status: HealthStatus,
    pub suggestions: Vec<String>,
    /// 文本列（Utf8 / LargeUtf8）的统计，非文本列为 `None`
    pub text_stats: Option<TextStats>,
}

/// 文本列统计。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextStats {
    /// 非空文本的平均长度（字符数）
    pub avg_length: f64,
    /// 是否检测到疑似 HTML 标签
    pub has_html: bool,
}

/// 健康评级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum HealthScore {
    Good,
    Warning,
    Danger,
}

/// 单列的健康状态（空值维度 + 重复维度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct HealthStatus {
    pub null_status: HealthScore,
    pub duplicate_status: HealthScore,
}

/// 人类可读的文件大小（如 `1.5 MB`），CLI 与 GUI 共用。
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}
