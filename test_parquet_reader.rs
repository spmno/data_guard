use anyhow::Result;
use data_guard::parser::parquet_reader::ParquetReader;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Test reading healthy file
    let healthy_path = PathBuf::from("test_healthy.parquet");
    let mut healthy_reader = ParquetReader::new(healthy_path)?;
    let (row_count, column_count, file_size) = healthy_reader.get_file_info()?;
    println!("Healthy file info:");
    println!("Row count: {}", row_count);
    println!("Column count: {}", column_count);
    println!("File size: {} bytes", file_size);
    println!("Schema: {}", healthy_reader.get_schema());

    // Test reading issues file
    let issues_path = PathBuf::from("test_issues.parquet");
    let mut issues_reader = ParquetReader::new(issues_path)?;
    let (row_count, column_count, file_size) = issues_reader.get_file_info()?;
    println!("\nIssues file info:");
    println!("Row count: {}", row_count);
    println!("Column count: {}", column_count);
    println!("File size: {} bytes", file_size);
    println!("Schema: {}", issues_reader.get_schema());

    // Test reading batches
    let batches = healthy_reader.read_batches(1000)?;
    println!("\nRead {} batches", batches.len());
    for (i, batch) in batches.iter().enumerate() {
        println!("Batch {}: {} rows", i, batch.num_rows());
    }

    Ok(())
}
