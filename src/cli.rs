use anyhow::Result;
use std::path::PathBuf;

use crate::parser::ParquetReader;
use crate::diagnosis::diagnose_batch;

pub fn run_cli(file_path: PathBuf) -> Result<()> {
    println!("Opening file: {:?}", file_path);
    
    let mut reader = ParquetReader::new(file_path)?;
    let (row_count, column_count, file_size) = reader.get_file_info()?;
    
    println!("\nFile Info:");
    println!("Row count: {}", row_count);
    println!("Column count: {}", column_count);
    println!("File size: {} bytes", file_size);
    println!("Schema: {}", reader.get_schema());
    
    println!("\nReading first 1000 rows for diagnosis...");
    let batches = reader.read_batches(1000)?;
    
    for batch in batches {
        println!("Processing batch with {} rows", batch.num_rows());
        let column_reports = diagnose_batch(&batch);
        
        println!("\nDiagnosis Report:");
        for report in column_reports {
            println!("\nColumn: {}", report.name);
            println!("  Null rate: {:.2}%", report.null_rate * 100.0);
            println!("  Duplicate rate: {:.2}%", report.duplicate_rate * 100.0);
            println!("  Unique count: {}", report.unique_count);
            println!("  Health status: Null={:?}, Duplicate={:?}", report.health_status.null_status, report.health_status.duplicate_status);
            if !report.suggestions.is_empty() {
                println!("  Suggestions:");
                for suggestion in report.suggestions {
                    println!("    - {}", suggestion);
                }
            }
            if let Some(text_stats) = report.text_stats {
                println!("  Text stats: Avg length={:.2}, Has HTML={}", text_stats.avg_length, text_stats.has_html);
            }
        }
    }
    
    Ok(())
}
