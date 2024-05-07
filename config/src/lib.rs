use std::env::current_dir;

use config::{Config, Environment};
use serde::Deserialize;
use tracing::{info, warn};

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub database_file: String,
}

impl AppConfig {
    pub fn from_env() -> Result<AppConfig, String> {
        let conf_builder = Config::builder()
            .add_source(Environment::with_prefix("app").try_parsing(true))
            .build()
            .expect("Failed to setup config builder.");

        let conf = conf_builder
            .try_deserialize::<AppConfig>()
            .map_err(|err| format!("Invalid config:\n{err:#?}"));
        conf
    }

    pub fn database_url(&self) -> String {
        format!("sqlite://{}?mode=rwc", self.database_file)
    }
}

pub fn load_env() -> Result<(), String> {
    let cwd = current_dir().map_err(|err| format!("Failed to access current directory: {err}"))?;
    let dotenv_path = cwd.join(".env");
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
