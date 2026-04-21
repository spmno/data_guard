use anyhow::Result;
use parquet::file::reader::{FileReader, SerializedFileReader};
use std::fs::File;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Test reading healthy file
    let healthy_path = PathBuf::from("test_healthy.parquet");
    let file = File::open(&healthy_path)?;
    let reader = SerializedFileReader::new(file)?;
    
    let metadata = reader.metadata();
    let row_count = metadata.file_metadata().num_rows();
    let column_count = metadata.file_metadata().schema().num_fields();
    let file_size = std::fs::metadata(&healthy_path)?.len();
    
    println!("Healthy file info:");
    println!("Row count: {}", row_count);
    println!("Column count: {}", column_count);
    println!("File size: {} bytes", file_size);
    println!("Schema: {:?}", metadata.file_metadata().schema());

    Ok(())
}
