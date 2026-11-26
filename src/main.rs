use std::process::ExitCode;

use clap::Parser as _;

use data_guard::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("错误: {err:#}");
            ExitCode::from(2)
        }
    }
}
