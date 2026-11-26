pub mod engine;

pub use engine::*;

use anyhow::Result;
use std::path::Path;

use crate::app::DiagnosisReport;
use crate::parser::ParquetReader;

/// 诊断一个 Parquet 文件：流式扫描全文件（或 `limit` 行），返回报告。
///
/// CLI 与 GUI 共用此入口。
pub fn diagnose_file(path: impl AsRef<Path>, limit: Option<usize>) -> Result<DiagnosisReport> {
    let reader = ParquetReader::open(path)?;
    let file_info = reader.file_info();
    let mut engine = DiagnosisEngine::new(reader.schema());
    for batch in reader.batch_iter(limit)? {
        engine.process_batch(&batch?)?;
    }
    Ok(engine.finish(file_info))
}
