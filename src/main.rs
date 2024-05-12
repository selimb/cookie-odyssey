use std::process::ExitCode;

use anyhow::Context;
use app_config::{load_env, AppConfig};
use tracing::info;

mod auth;
mod journal;
mod router;
mod server;
mod storage;
mod template_engine;
mod utils;

#[tokio::main]
async fn run() -> Result<(), anyhow::Error> {
    init_tracing()?;
    load_env()?;
    let conf = AppConfig::from_env()?;
    let app = crate::server::mkapp(&conf).await?;

    let port = 3000;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .context("Failed to bind TCP listener")?;
    let addr = format!("http://localhost:{port}");
    info!("Starting server on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() -> Result<(), anyhow::Error> {
    let format = tracing_subscriber::fmt::format().without_time();
    let subscriber = tracing_subscriber::fmt::fmt().event_format(format).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:?}");
            ExitCode::FAILURE
        }
    }
}
