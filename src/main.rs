use std::process::ExitCode;

use app_config::{load_env, AppConfig};
use tracing::info;

mod routes;
mod server;

#[tokio::main]
async fn run() -> Result<(), String> {
    init_tracing()?;
    load_env()?;
    let conf = AppConfig::from_env()?;
    let app = crate::server::mkapp(conf).await?;

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
