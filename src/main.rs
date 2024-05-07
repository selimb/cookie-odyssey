use std::{env::current_dir, process::ExitCode};

use tracing::{info, warn};

use crate::config::AppConfig;

mod config;
mod routes;
mod server;

#[tokio::main]
async fn run() -> Result<(), String> {
    init_tracing()?;
    load_env()?;
    let conf = AppConfig::from_env()?;
    let app = crate::server::mkapp(conf)?;

    let port = 3000;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .map_err(|err| err.to_string())?;
    let addr = format!("http://localhost:{port}");
    info!("Starting server on {addr}");
    axum::serve(listener, app)
        .await
        .map_err(|err| err.to_string())
}

fn load_env() -> Result<(), String> {
    let cwd = current_dir().map_err(|err| format!("Failed to access current directory: {err}"))?;
    let dotenv_path = cwd.join("config").join(".env");
    if dotenv_path.exists() {
        match dotenv::from_path(&dotenv_path) {
            Ok(_) => {
                let p = dotenv_path.to_string_lossy();
                info!("Loaded environment from: {p}");
            }
            Err(x) => {
                return Err(x.to_string());
            }
        }
    } else {
        let p = dotenv_path.to_string_lossy();
        warn!("No environment file found at: {p}");
    }
    Ok(())
}

fn init_tracing() -> Result<(), String> {
    let format = tracing_subscriber::fmt::format().without_time();
    let subscriber = tracing_subscriber::fmt::fmt().event_format(format).finish();
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
