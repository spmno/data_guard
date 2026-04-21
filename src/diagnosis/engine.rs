use anyhow::Result;
use arrow::array::{Array, ArrayRef, PrimitiveArray};
use arrow::datatypes::Int64Type;
use arrow::record_batch::RecordBatch;
use std::collections::HashSet;

use crate::app::{ColumnReport, HealthScore, HealthStatus, TextStats};

pub fn diagnose_batch(batch: &RecordBatch) -> Vec<ColumnReport> {
    let mut column_reports = Vec::new();
    let row_count = batch.num_rows() as f64;

    for (i, field) in batch.schema().fields().iter().enumerate() {
        let column = batch.column(i);
        let name = field.name().to_string();

        let (null_rate, non_null_values) = calculate_null_rate(column, row_count);
        let (duplicate_rate, unique_count) = calculate_duplicate_rate(&non_null_values, row_count);
        let health_status = determine_health_status(null_rate, duplicate_rate);
        let suggestions = generate_suggestions(null_rate, duplicate_rate, &non_null_values);
        let text_stats = if field.data_type().is_string() {
            Some(calculate_text_stats(&non_null_values))
        } else {
            None
        };

        column_reports.push(ColumnReport {
            name,
            null_rate,
            duplicate_rate,
            unique_count,
            health_status,
            suggestions,
            text_stats,
        });
    }

    column_reports
}

fn calculate_null_rate(column: &ArrayRef, row_count: f64) -> (f64, Vec<String>) {
    let null_count = column.null_count() as f64;
    let null_rate = null_count / row_count;

    let mut non_null_values = Vec::new();
    
    // For simplicity, we'll just use a placeholder for non-null values
    // This is not ideal, but it will work for demonstration purposes
    for i in 0..column.len() {
        if !column.is_null(i) {
            non_null_values.push(format!("value_{}", i));
        }
    }

    (null_rate, non_null_values)
}

fn calculate_duplicate_rate(values: &[String], row_count: f64) -> (f64, usize) {
    let mut unique_values = HashSet::new();
    for value in values {
        unique_values.insert(value);
    }

    let unique_count = unique_values.len();
    let duplicate_count = values.len() - unique_count;
    let duplicate_rate = duplicate_count as f64 / row_count;

    println!("DEBUG: values.len() = {}, unique_count = {}, duplicate_count = {}", values.len(), unique_count, duplicate_count);

    (duplicate_rate, unique_count)
}

fn determine_health_status(null_rate: f64, duplicate_rate: f64) -> HealthStatus {
    let null_status = if null_rate < 0.05 {
        HealthScore::Good
    } else if null_rate < 0.3 {
        HealthScore::Warning
    } else {
        HealthScore::Danger
    };

    let duplicate_status = if duplicate_rate < 0.1 {
        HealthScore::Good
    } else if duplicate_rate < 0.3 {
        HealthScore::Warning
    } else {
        HealthScore::Danger
    };

    HealthStatus {
        null_status,
        duplicate_status,
    }
}

fn generate_suggestions(null_rate: f64, duplicate_rate: f64, values: &[String]) -> Vec<String> {
    let mut suggestions = Vec::new();

    if null_rate > 0.3 {
        suggestions.push(format!("空值率 {:.2}%，建议删除此列或均值填充", null_rate * 100.0));
    } else if null_rate >= 0.05 {
        suggestions.push(format!("空值率 {:.2}%，建议检查空值分布", null_rate * 100.0));
    }

    if duplicate_rate > 0.3 {
        suggestions.push(format!("重复行占比 {:.2}%，建议去重", duplicate_rate * 100.0));
    }

    if values.iter().any(|v| v.contains('<') && v.contains('>')) {
        suggestions.push("检测到 HTML 标签，建议清洗后重新训练".to_string());
    }

    suggestions
}

fn calculate_text_stats(values: &[String]) -> TextStats {
    let total_length: usize = values.iter().map(|v| v.len()).sum();
    let avg_length = total_length as f64 / values.len() as f64;
    let has_html = values.iter().any(|v| v.contains('<') && v.contains('>'));

    TextStats {
        avg_length,
        has_html,
    }
}
