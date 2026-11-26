//! 命令行入口：参数解析、扫描执行、报告输出（人类可读 / JSON）。

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use crate::app::{DiagnosisReport, HealthScore};
use crate::diagnosis::DiagnosisEngine;
use crate::parser::ParquetReader;

/// Parquet 数据质量诊断工具：扫描列级空值率 / 重复率 / 文本健康度，输出修复建议
#[derive(Parser, Debug)]
#[command(
    name = "data_guard",
    version,
    about,
    long_about = None,
    after_help = "退出码：0 = 健康（允许含警告）  1 = 存在危险列  2 = 运行错误"
)]
pub struct Cli {
    /// 要诊断的 Parquet 文件路径
    pub file: PathBuf,

    /// 最多扫描的行数（默认扫描全文件）
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// 以 JSON 输出完整报告（便于接入数据流水线）
    #[arg(long)]
    pub json: bool,
}

impl Cli {
    /// 执行诊断。
    ///
    /// 返回进程退出码：0 健康、1 存在危险列、2 由调用方在 `Err` 时给出。
    pub fn run(&self) -> Result<ExitCode> {
        let reader = ParquetReader::open(&self.file)?;
        let file_info = reader.file_info();
        let mut engine = DiagnosisEngine::new(reader.schema());

        for batch in reader.batch_iter(self.limit)? {
            engine.process_batch(&batch?)?;
        }

        let report = engine.finish(file_info);

        if self.json {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{json}");
        } else {
            print_report(&report);
        }

        if report.overall_score == HealthScore::Danger {
            Ok(ExitCode::from(1))
        } else {
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn print_report(report: &DiagnosisReport) {
    let info = &report.file_info;
    println!("data_guard 诊断报告");
    println!(
        "文件: {} ({})",
        info.path.display(),
        human_size(info.file_size)
    );
    println!(
        "行数: {}（扫描 {}） | 列数: {} | 总体评级: {}",
        info.row_count,
        report.scanned_rows,
        info.column_count,
        health_label(report.overall_score)
    );
    if (report.scanned_rows as i64) < info.row_count {
        println!("注意: 受 --limit 限制仅扫描了部分数据，统计结果不代表全文件");
    }
    println!();

    let total = report.column_reports.len();
    for (index, col) in report.column_reports.iter().enumerate() {
        println!("[{}/{}] {} ({})", index + 1, total, col.name, col.data_type);
        println!(
            "    空值率  {:>8.2}%  [{}]",
            col.null_rate * 100.0,
            health_label(col.health_status.null_status)
        );
        println!(
            "    重复率  {:>8.2}%  [{}]",
            col.duplicate_rate * 100.0,
            health_label(col.health_status.duplicate_status)
        );
        println!(
            "    唯一值  {}（非空 {} / 总行 {}）",
            col.unique_count, col.non_null_count, report.scanned_rows
        );
        if let Some(text) = &col.text_stats {
            println!(
                "    文本统计  平均长度 {:.2}，{}",
                text.avg_length,
                if text.has_html {
                    "检测到 HTML 标签"
                } else {
                    "无 HTML 标签"
                }
            );
        }
        if !col.suggestions.is_empty() {
            println!("    建议:");
            for suggestion in &col.suggestions {
                println!("      - {suggestion}");
            }
        }
        println!();
    }

    println!(
        "汇总: {} 列中 {} 列危险、{} 列警告",
        total, report.danger_columns, report.warning_columns
    );
}

fn health_label(score: HealthScore) -> &'static str {
    match score {
        HealthScore::Good => "良好",
        HealthScore::Warning => "警告",
        HealthScore::Danger => "危险",
    }
}

fn human_size(bytes: u64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_formats() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }
}
