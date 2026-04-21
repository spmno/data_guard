mod app;
mod diagnosis;
mod parser;
mod cli;

use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: data_guard <parquet_file>");
        return Ok(());
    }
    
    let file_path = std::path::PathBuf::from(&args[1]);
    cli::run_cli(file_path)?;
    
    Ok(())
}

