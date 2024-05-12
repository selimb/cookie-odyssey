use anyhow::Context;
use config::{Config, Environment};
use serde::Deserialize;
use std::env::current_dir;
use thiserror::Error;
use tracing::{info, warn};

pub fn load_env() -> Result<(), anyhow::Error> {
    let cwd = current_dir().context("Failed to access current directory")?;
    let dotenv_path = cwd.join(".env");
    if dotenv_path.exists() {
        dotenv::from_path(&dotenv_path)?;
        let p = dotenv_path.to_string_lossy();
        info!("Loaded environment from: {p}");
    } else {
        let p = dotenv_path.to_string_lossy();
        warn!("No environment file found at: {p}");
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub database_file: String,
    pub storage: StorageConfig,
}

/// S3/R2 config
#[derive(Clone, Debug, Deserialize)]
pub struct StorageConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    pub endpoint: String,
    pub allow_http: bool,
}

impl AppConfig {
    pub fn from_env() -> Result<AppConfig, ConfigError> {
        let conf_builder = Config::builder()
            .add_source(
                Environment::with_prefix("APP")
                    .separator(".")
                    .try_parsing(true),
            )
            .build()
            .expect("Failed to setup config builder.");

        let conf = conf_builder.try_deserialize::<AppConfig>()?;
        Ok(conf)
    }

    pub fn database_url(&self) -> String {
        format!("sqlite://{}?mode=rwc", self.database_file)
    }
}

#[derive(Error, Debug)]
#[error("Invalid config:\n{0:#?}")]
pub struct ConfigError(#[from] config::ConfigError);
