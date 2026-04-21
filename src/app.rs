use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug)]
pub struct AppState {
    pub file_path: Option<PathBuf>,
    pub file_info: Option<FileInfo>,
    pub diagnosis_report: Option<DiagnosisReport>,
}

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub row_count: i64,
    pub column_count: usize,
    pub file_size: u64,
}

#[derive(Debug)]
pub struct DiagnosisReport {
    pub overall_score: HealthScore,
    pub file_info: FileInfo,
    pub column_reports: Vec<ColumnReport>,
}

#[derive(Debug)]
pub struct ColumnReport {
    pub name: String,
    pub null_rate: f64,
    pub duplicate_rate: f64,
    pub unique_count: usize,
    pub health_status: HealthStatus,
    pub suggestions: Vec<String>,
    pub text_stats: Option<TextStats>,
}

#[derive(Debug)]
pub struct TextStats {
    pub avg_length: f64,
    pub has_html: bool,
}

#[derive(Debug, PartialEq)]
pub enum HealthScore {
    Good,
    Warning,
    Danger,
}

#[derive(Debug, PartialEq)]
pub struct HealthStatus {
    pub null_status: HealthScore,
    pub duplicate_status: HealthScore,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            file_path: None,
            file_info: None,
            diagnosis_report: None,
        }
    }
}
