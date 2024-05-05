use std::process::ExitCode;

use crate::config::AppConfig;

mod config;

fn run() -> Result<(), String> {
    dotenv::dotenv().map_err(|err| err.to_string())?;
    init_tracing()?;
    let conf = AppConfig::from_env()?;

    println!("Hello, world!");

    Ok(())
}

fn init_tracing() -> Result<(), String> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).map_err(|err| err.to_string())
}

fn main() -> ExitCode {
    match run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
