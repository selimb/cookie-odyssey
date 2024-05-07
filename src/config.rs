use config::{Config, Environment};
use serde::Deserialize;

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
}
